//! PhotonCast App Management Library
//!
//! This crate provides app management features for PhotonCast, including:
//!
//! - App uninstaller with deep scan and file selection
//! - Force quit for running applications
//! - Auto quit feature to automatically quit idle apps
//! - App actions (show in Finder, copy path/bundle ID, hide)
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

pub mod actions;
pub mod auto_quit;
pub mod bundle;
pub mod config;
pub mod error;
pub mod models;
pub mod process;
pub mod scanner;
pub mod sleep;
pub mod uninstaller;

pub mod commands;

pub use actions::{
    copy_bundle_id_to_clipboard, copy_path_to_clipboard, hide_app, reveal_in_finder, ActionError,
    ActionResult,
};
pub use auto_quit::{
    get_suggested_app_name, is_suggested_auto_quit_app, suggested_auto_quit_apps,
    AutoQuitAppConfig, AutoQuitConfig, AutoQuitManager, SUGGESTED_AUTO_QUIT_APPS,
    DEFAULT_AUTO_QUIT_TIMEOUT_MINUTES,
};
pub use bundle::{format_size, is_protected_app, is_system_app, is_system_app_by_bundle_id};
pub use scanner::find_group_containers;
pub use uninstaller::{calculate_selected_size, get_selected_files, uninstall_selected};
pub use process::{
    force_quit_app_action, get_frontmost_app_bundle_id, get_running_apps_detailed,
    is_app_responding, is_app_running, quit_app_by_bundle_id, quit_app_with_timeout,
    should_confirm_force_quit,
};
pub use config::{AppSleepConfig, AppSleepOverride, AppsConfig};
pub use error::{AppError, Result};
pub use models::{
    Application, RelatedFile, RelatedFileCategory, RunningApp, RunningApplication, UninstallPreview,
};
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

    /// Performs the uninstall with explicit file selection.
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

    /// Performs the uninstall, respecting the `selected` field on related files.
    ///
    /// Only removes files where `selected == true`. All files default to selected.
    ///
    /// # Errors
    ///
    /// Returns an error if the uninstall fails.
    pub fn uninstall_selected(&self, preview: &UninstallPreview) -> Result<()> {
        uninstaller::uninstall_selected(preview)
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
