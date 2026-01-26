//! Application lifecycle and state management.
//!
//! This module contains the core application state, configuration,
//! and action definitions for PhotonCast.

pub mod actions;
pub mod config;
pub mod config_file;
pub mod integration;
pub mod keybindings;
pub mod state;

pub use actions::*;
pub use config::Config;
pub use config_file::{
    default_config_dir, default_config_path, ensure_config_dir, ensure_config_file, load_config,
    load_config_from, save_config, save_config_to, ConfigFileError, ConfigManager, ConfigResult,
};
pub use integration::{
    ExtensionLaunchError, IntegrationConfig, PhotonCastApp, SearchOutcome, SEARCH_TIMEOUT_MESSAGE,
};
pub use keybindings::{
    default_keybindings_path, Keybindings, KeybindingsError, KeybindingsResult, Shortcut,
};
pub use state::AppState;
