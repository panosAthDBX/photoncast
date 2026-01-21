//! PhotonCast Calendar Integration Library
//!
//! This crate provides calendar integration for PhotonCast, including:
//!
//! - EventKit integration for macOS calendar access
//! - Conference URL detection (Zoom, Google Meet, Microsoft Teams)
//! - Event fetching and filtering
//! - Permission management
//!
//! # Example
//!
//! ```rust,ignore
//! use photoncast_calendar::{CalendarCommand, CalendarCommandType};
//!
//! // Create calendar command
//! let command = CalendarCommand::with_default_config();
//!
//! // Request permission
//! command.request_permission()?;
//!
//! // Fetch today's events
//! let events = command.fetch_today_events().await?;
//! ```

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]

pub mod conference;
pub mod config;
pub mod error;
pub mod models;

#[cfg(target_os = "macos")]
pub mod eventkit;

pub mod commands;

pub use conference::{detect_conference_url, detect_provider, ConferenceProvider};
pub use config::CalendarConfig;
pub use error::{CalendarError, Result};
pub use models::{Attendee, CalendarEvent};

// Re-export chrono for use in dependent crates
pub use chrono;

#[cfg(target_os = "macos")]
pub use eventkit::{EventKitManager, PermissionStatus};

pub use commands::{CalendarAction, CalendarCommand, CalendarCommandInfo, CalendarCommandType};

/// The main calendar manager.
///
/// Coordinates calendar operations and configuration.
#[derive(Debug)]
pub struct CalendarManager {
    /// Configuration.
    config: CalendarConfig,
}

impl CalendarManager {
    /// Creates a new calendar manager with the given configuration.
    #[must_use]
    pub const fn new(config: CalendarConfig) -> Self {
        Self { config }
    }

    /// Gets the current configuration.
    #[must_use]
    pub const fn config(&self) -> &CalendarConfig {
        &self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: CalendarConfig) {
        self.config = config;
    }
}

impl Default for CalendarManager {
    fn default() -> Self {
        Self::new(CalendarConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_manager_creation() {
        let manager = CalendarManager::new(CalendarConfig::default());
        assert!(manager.config().enabled);
    }

    #[test]
    fn test_config_update() {
        let mut manager = CalendarManager::new(CalendarConfig::default());

        let new_config = CalendarConfig {
            enabled: false,
            ..Default::default()
        };

        manager.set_config(new_config);
        assert!(!manager.config().enabled);
    }
}
