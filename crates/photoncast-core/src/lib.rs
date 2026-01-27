#![recursion_limit = "1024"]
//! PhotonCast Core Library
//!
//! This crate contains the core functionality for PhotonCast, a lightning-fast
//! macOS launcher built in pure Rust.
//!
//! # Modules
//!
//! - [`app`] - Application lifecycle and state management
//! - [`ui`] - GPUI views and components
//! - [`search`] - Search engine and providers
//! - [`indexer`] - Application indexing and scanning
//! - [`storage`] - Database and persistence
//! - [`platform`] - macOS-specific integrations
//! - [`theme`] - Catppuccin theming system
//! - [`commands`] - System commands
//! - [`utils`] - Shared utilities

// Clippy configuration: warn on all + pedantic, with targeted project-wide exceptions
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
// Project-wide style choices (kept as blanket allows)
#![allow(clippy::module_name_repetitions)] // Common Rust naming pattern
#![allow(clippy::must_use_candidate)] // Too noisy for this codebase
#![allow(clippy::missing_errors_doc)] // Documentation will be added incrementally
#![allow(clippy::missing_panics_doc)] // Documentation will be added incrementally
#![allow(clippy::struct_excessive_bools)] // UI state often needs many bools
#![allow(clippy::type_complexity)] // Complex types acceptable for GPUI elements
#![allow(clippy::too_many_lines)] // Pragmatic choice for complex functions
#![allow(clippy::too_many_arguments)] // Pragmatic choice for builder-style APIs
#![allow(clippy::cast_possible_wrap)] // Pervasive in timestamp/DB conversions (u64↔i64)
#![allow(clippy::cast_sign_loss)] // Pervasive in validated timestamp/size conversions
#![allow(clippy::significant_drop_tightening)] // MutexGuard usage patterns are intentional
#![allow(clippy::await_holding_lock)] // Intentional for async icon loading with cache
#![allow(clippy::doc_markdown)] // Will fix documentation incrementally
#![allow(clippy::wildcard_imports)] // Used for prelude-style imports in GPUI
#![allow(dead_code)] // Expected in placeholder/evolving code

pub mod app;
pub mod commands;
pub mod custom_commands;
pub mod extensions;
pub mod indexer;
pub mod platform;
pub mod search;
pub mod storage;
pub mod utils;

// UI and theme modules require GPUI (disabled during tests due to macro expansion depth)
#[cfg(all(feature = "ui", not(test)))]
pub mod theme;
#[cfg(all(feature = "ui", not(test)))]
pub mod ui;

#[cfg(all(feature = "ui", test))]
pub mod theme {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum CatppuccinFlavor {
        Latte,
        Frappe,
        Macchiato,
        #[default]
        Mocha,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum AccentColor {
        Rosewater,
        Flamingo,
        Pink,
        Mauve,
        Red,
        Maroon,
        Peach,
        Yellow,
        Green,
        Teal,
        Sky,
        Sapphire,
        #[default]
        Blue,
        Lavender,
    }

    #[derive(Debug, Default, Clone)]
    pub struct ThemeColors;
}

#[cfg(all(feature = "ui", test))]
pub mod ui {
    pub mod permission_dialog {
        use crate::platform::accessibility::PermissionStatus;

        #[derive(Debug, Clone)]
        pub struct PermissionDialog {
            pub status: PermissionStatus,
            pub is_polling: bool,
            pub is_visible: bool,
        }

        impl PermissionDialog {
            #[must_use]
            pub const fn new() -> Self {
                Self {
                    status: PermissionStatus::Unknown,
                    is_polling: false,
                    is_visible: false,
                }
            }

            pub fn show(&mut self) {
                self.is_visible = true;
            }

            pub fn hide(&mut self) {
                self.is_visible = false;
                self.is_polling = false;
            }
        }

        impl Default for PermissionDialog {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

/// Re-export commonly used types at the crate root.
pub mod prelude {
    pub use crate::app::{config::Config, state::AppState};
    pub use crate::search::{SearchEngine, SearchResult};
    #[cfg(feature = "ui")]
    pub use crate::theme::{CatppuccinFlavor, ThemeColors};
}
