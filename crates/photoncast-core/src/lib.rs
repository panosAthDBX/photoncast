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
#![allow(dead_code)] // Expected in placeholder code

pub mod app;
pub mod commands;
pub mod indexer;
pub mod platform;
pub mod search;
pub mod storage;
pub mod utils;

// UI and theme modules require GPUI (disabled during tests due to macro expansion depth)
#[cfg(feature = "ui")]
pub mod theme;
#[cfg(feature = "ui")]
pub mod ui;

/// Re-export commonly used types at the crate root.
pub mod prelude {
    pub use crate::app::{config::Config, state::AppState};
    pub use crate::search::{SearchEngine, SearchResult};
    #[cfg(feature = "ui")]
    pub use crate::theme::{CatppuccinFlavor, ThemeColors};
}
