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

// Enable warnings for actual implementation, allow certain lints for placeholder code
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
// Allow certain lints that are expected in placeholder/stub implementations
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)] // Will add const where appropriate during implementation
#![allow(clippy::unused_async)] // Async functions will have await once implemented
#![allow(clippy::map_unwrap_or)] // Style preference, will refactor during implementation
#![allow(clippy::struct_excessive_bools)] // Will be refactored if needed
#![allow(clippy::derivable_impls)] // Some Default impls have comments
#![allow(clippy::doc_markdown)] // Will fix documentation during implementation
#![allow(clippy::should_implement_trait)] // Will implement traits during development
#![allow(clippy::match_same_arms)] // Placeholder implementations
#![allow(clippy::cast_precision_loss)] // Acceptable for frecency calculations
#![allow(clippy::unnecessary_literal_bound)] // Will fix during implementation
#![allow(clippy::uninlined_format_args)] // Style preference
#![allow(clippy::case_sensitive_file_extension_comparisons)] // Will fix during implementation
#![allow(clippy::missing_errors_doc)] // Will add error docs during implementation
#![allow(clippy::must_use_candidate)] // Will add #[must_use] where appropriate
#![allow(clippy::wildcard_imports)] // Used for prelude-style imports
#![allow(clippy::unused_self)] // Some methods will use self in future
#![allow(clippy::single_match_else)] // Style preference for explicit matching
#![allow(clippy::type_complexity)] // Complex types acceptable for GPUI elements
#![allow(clippy::redundant_closure_for_method_calls)] // Style preference
#![allow(clippy::cast_possible_wrap)] // Acceptable for timestamp conversions
#![allow(clippy::cast_sign_loss)] // Acceptable for validated conversions
#![allow(clippy::needless_pass_by_value)] // Some APIs require owned values
#![allow(clippy::if_not_else)] // Style preference
#![allow(clippy::match_wildcard_for_single_variants)] // Explicit matching preferred
#![allow(clippy::significant_drop_tightening)] // MutexGuard across await is intentional
#![allow(clippy::option_map_or_none)] // Style preference
#![allow(clippy::useless_format)] // Will fix during implementation
#![allow(clippy::double_must_use)] // Acceptable for wrapper types
#![allow(clippy::manual_let_else)] // Style preference
#![allow(clippy::if_same_then_else)] // Placeholder implementations
#![allow(clippy::doc_overindented_list_items)] // Will fix docs later
#![allow(clippy::cast_possible_truncation)] // Validated at runtime
#![allow(clippy::assigning_clones)] // Style preference
#![allow(clippy::option_if_let_else)] // Style preference for explicit matching
#![allow(clippy::redundant_closure)] // Style preference
#![allow(clippy::manual_filter_map)] // Style preference
#![allow(clippy::match_wild_err_arm)] // Explicit error handling style
#![allow(clippy::unit_arg)] // Matching over () is intentional for Result handling
#![allow(clippy::await_holding_lock)] // Intentional for async icon loading with cache
#![allow(dead_code)] // Expected in placeholder code

pub mod app;
pub mod commands;
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
