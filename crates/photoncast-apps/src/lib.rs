//! PhotonCast App Management Library
//!
//! This crate provides app management features for PhotonCast, including:
//!
//! - App uninstaller with deep scan
//! - Force quit for running applications
//! - App sleep feature to stop idle apps
//!
//! # Example
//!
//! ```rust,ignore
//! use photoncast_apps::{AppManager, AppsConfig};
//!
//! // Create manager
//! let manager = AppManager::new(AppsConfig::default());
//!
//! // Create uninstall preview
//! let preview = manager.create_uninstall_preview("/Applications/MyApp.app", true).await?;
//! ```

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::uninlined_format_args)]

pub mod bundle;
pub mod config;
pub mod error;
pub mod models;
pub mod process;
pub mod scanner;
pub mod sleep;
pub mod uninstaller;

pub mod commands;

pub use config::{AppSleepConfig, AppSleepOverride, AppsConfig};
pub use error::{AppError, Result};
pub use models::{Application, RelatedFile, RelatedFileCategory, RunningApp, UninstallPreview};
pub use sleep::AppSleepManager;

/// The main app manager.
///
/// Coordinates all app management operations.
#[derive(Debug)]
pub struct AppManager {
    /// Configuration.
    config: AppsConfig,
    /// App sleep manager.
    sleep_manager: AppSleepManager,
}

impl AppManager {
    /// Creates a new app manager with the given configuration.
    #[must_use]
    pub fn new(config: AppsConfig) -> Self {
        let sleep_manager = AppSleepManager::new(config.app_sleep.clone());

        Self {
            config,
            sleep_manager,
        }
    }

    /// Returns the configuration.
    #[must_use]
    pub const fn config(&self) -> &AppsConfig {
        &self.config
    }

    /// Returns the app sleep manager.
    #[must_use]
    pub const fn sleep_manager(&self) -> &AppSleepManager {
        &self.sleep_manager
    }

    /// Creates an uninstall preview for an app.
    ///
    /// # Errors
    ///
    /// Returns an error if the preview cannot be created.
    pub fn create_uninstall_preview(&self, app_path: &std::path::Path) -> Result<UninstallPreview> {
        uninstaller::create_uninstall_preview(app_path, self.config.deep_scan_default)
    }

    /// Performs the uninstall.
    ///
    /// # Errors
    ///
    /// Returns an error if the uninstall fails.
    pub fn uninstall(
        &self,
        preview: &UninstallPreview,
        selected_files: &[&RelatedFile],
    ) -> Result<()> {
        uninstaller::uninstall(preview, selected_files)
    }

    /// Gets a list of running applications.
    ///
    /// # Errors
    ///
    /// Returns an error if enumeration fails.
    pub fn get_running_apps(&self) -> Result<Vec<RunningApp>> {
        process::get_running_apps()
    }

    /// Gracefully quits an application.
    ///
    /// # Errors
    ///
    /// Returns an error if the quit fails.
    pub fn quit_app(&self, pid: u32) -> Result<()> {
        process::quit_app(pid)
    }

    /// Force quits an application.
    ///
    /// # Errors
    ///
    /// Returns an error if the force quit fails.
    pub fn force_quit_app(&self, pid: u32) -> Result<()> {
        process::force_quit_app(pid)
    }
}
