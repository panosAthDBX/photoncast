//! PhotonCast Quick Links Library
//!
//! This crate provides quick links management for PhotonCast, including:
//!
//! - SQLite storage with FTS5 search
//! - TOML export/import for backup and sharing
//! - Browser bookmark import (Safari, Chrome, Firefox, Arc)
//! - Favicon fetching and caching
//! - Dynamic URL support with {query} placeholders
//!
//! # Example
//!
//! ```rust,ignore
//! use photoncast_quicklinks::{QuickLink, QuickLinksStorage};
//!
//! // Create storage
//! let storage = QuickLinksStorage::open("quicklinks.db").await?;
//!
//! // Create and store a link
//! let link = QuickLink::new("GitHub", "https://github.com");
//! storage.store(&link).await?;
//!
//! // Search links
//! let results = storage.search("git").await?;
//! ```

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]

pub mod browser_import;
pub mod config;
pub mod error;
pub mod favicon;
pub mod library;
pub mod models;
pub mod placeholder;
pub mod storage;
pub mod toml_io;

#[cfg(feature = "ui")]
pub mod ui;

pub use config::QuickLinksConfig;
pub use error::{QuickLinksError, Result};
pub use library::{
    get_bundled_quicklinks, get_by_category, get_categories, to_quicklink, BundledQuickLink,
    BUNDLED_QUICKLINKS,
};
pub use models::{QuickLink, QuickLinkIcon, QuickLinkId, QuickLinkToml, QuickLinksToml};
pub use storage::QuickLinksStorage;
