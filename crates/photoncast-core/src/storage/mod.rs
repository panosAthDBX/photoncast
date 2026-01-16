//! Database and persistence.
//!
//! This module handles storage of usage data, app cache, and settings.
//!
//! # Architecture
//!
//! The storage layer uses SQLite with async wrappers via `tokio::task::spawn_blocking`
//! to avoid blocking the async runtime. The database is stored at:
//!
//! `~/Library/Application Support/PhotonCast/photoncast.db`
//!
//! # Schema
//!
//! The database uses a versioned migration system. Current schema (v1) includes:
//!
//! - `schema_version` - Tracks applied migrations
//! - `app_usage` - Tracks app launch frequency for frecency ranking
//! - `command_usage` - Tracks command execution frequency
//! - `file_usage` - Tracks file access frequency
//! - `app_cache` - Caches indexed application metadata
//!
//! # Example
//!
//! ```rust,ignore
//! use photoncast_core::storage::{Database, UsageTracker, default_database_path};
//!
//! // Open database at default location
//! let db = Database::open_async(default_database_path()).await?;
//!
//! // Create usage tracker for frecency
//! let tracker = UsageTracker::new(db.clone());
//!
//! // Record app launch
//! tracker.record_app_launch_async("com.apple.Safari".to_string()).await?;
//!
//! // Get frecency score
//! let frecency = tracker.get_app_frecency_async("com.apple.Safari".to_string()).await?;
//! ```

pub mod database;
pub mod usage;

pub use database::{default_database_path, Database};
pub use usage::UsageTracker;
