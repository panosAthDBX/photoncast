//! PhotonCast Sleep Timer Module
//!
//! This crate provides sleep timer functionality for PhotonCast, including:
//!
//! - Timer scheduling with SQLite persistence
//! - Natural language parsing for timer expressions
//! - System action execution (sleep, shutdown, restart, lock)
//!
//! # Example
//!
//! ```no_run
//! use photoncast_timer::{TimerScheduler, parser::parse_timer_expression};
//!
//! #[tokio::main]
//! async fn main() {
//!     let scheduler = TimerScheduler::new("timer.db").await.unwrap();
//!
//!     // Parse natural language expression
//!     let expr = parse_timer_expression("sleep in 30 minutes").unwrap();
//!     
//!     // Create and set timer
//!     let timer = photoncast_timer::ActiveTimer::new(expr.action, expr.execute_at);
//!     scheduler.set_timer(timer).await.unwrap();
//! }
//! ```

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::arc_with_non_send_sync)] // Timer uses Arc<RwLock<Connection>> for async access

pub mod config;
pub mod error;
pub mod parser;
pub mod scheduler;

pub mod commands;

#[cfg(feature = "ui")]
pub mod ui;

pub use config::TimerConfig;
pub use error::{Result, TimerError};
pub use parser::{parse_timer_expression, TimerExpression};
pub use scheduler::{ActiveTimer, TimerAction, TimerScheduler};
