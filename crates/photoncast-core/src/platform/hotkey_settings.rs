//! Hotkey settings and customization.
//!
//! This module provides functionality for customizing the global hotkey,
//! including key capture, binding validation, and configuration persistence.

use std::fmt;
use std::path::Path;

use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::app::config::{Config, HotkeyConfig};
use crate::platform::hotkey::{
    detect_hotkey_conflict, HotkeyBinding, HotkeyError, HotkeyManager, Modifier, Modifiers,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during hotkey settings operations.
#[derive(Error, Debug, Clone)]
pub enum HotkeySettingsError {
    /// The binding is invalid.
    #[error("invalid hotkey binding: {reason}")]
    InvalidBinding {
        /// Reason why the binding is invalid.
        reason: String,
    },

    /// Failed to save configuration.
    #[error("failed to save configuration: {reason}")]
    SaveFailed {
        /// Reason for the failure.
        reason: String,
    },

    /// Failed to load configuration.
    #[error("failed to load configuration: {reason}")]
    LoadFailed {
        /// Reason for the failure.
        reason: String,
    },

    /// The key is reserved and cannot be used.
    #[error("reserved key: {key} cannot be used as a hotkey")]
    ReservedKey {
        /// The reserved key.
        key: String,
    },

    /// A single modifier cannot be used as a hotkey.
    #[error("a modifier key alone cannot be used as a hotkey (use double-tap instead)")]
    SingleModifier,

    /// Hotkey registration failed.
    #[error("hotkey registration failed: {0}")]
    RegistrationFailed(#[from] HotkeyError),
}

impl HotkeySettingsError {
    /// Returns a user-friendly message for this error.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::InvalidBinding { reason } => {
                format!("The hotkey you entered is not valid: {}", reason)
            },
            Self::SaveFailed { reason } => {
                format!("Failed to save your hotkey settings: {}", reason)
            },
            Self::LoadFailed { reason } => {
                format!("Failed to load hotkey settings: {}", reason)
            },
            Self::ReservedKey { key } => {
                format!(
                    "The key '{}' is reserved by the system and cannot be used as a hotkey. \
                     Please choose a different key.",
                    key
                )
            },
            Self::SingleModifier => {
                "A modifier key alone (like Command or Option) cannot be used as a hotkey. \
                 Either combine it with another key, or enable double-tap mode."
                    .to_string()
            },
            Self::RegistrationFailed(err) => err.user_message(),
        }
    }
}

// =============================================================================
// Reserved Keys
// =============================================================================

/// Keys that are reserved by the system and cannot be used as hotkeys.
const RESERVED_KEYS: &[&str] = &[
    // System keys
    "Power", "Eject", "CapsLock",
    // Function keys that have special system bindings
    "F1",  // Brightness down (on many Macs)
    "F2",  // Brightness up (on many Macs)
    "F3",  // Mission Control (on many Macs)
    "F4",  // Launchpad (on many Macs)
    "F10", // Mute (on many Macs)
    "F11", // Volume down (on many Macs)
    "F12", // Volume up (on many Macs)
];

/// Checks if a key is reserved.
#[must_use]
pub fn is_reserved_key(key: &str) -> bool {
    RESERVED_KEYS.iter().any(|&k| k.eq_ignore_ascii_case(key))
}

// =============================================================================
// Binding Validation
// =============================================================================

/// Validates a hotkey binding.
///
/// # Errors
///
/// Returns an error if:
/// - The key is empty
/// - The key is reserved by the system
/// - No modifiers are provided (single key hotkeys are not allowed)
/// - A modifier is used as the main key without double-tap mode
pub fn validate_binding(
    key: &str,
    modifiers: &[String],
    allow_double_tap: bool,
) -> Result<(), HotkeySettingsError> {
    // Check for empty key
    if key.is_empty() {
        return Err(HotkeySettingsError::InvalidBinding {
            reason: "no key specified".to_string(),
        });
    }

    // Check for reserved keys
    if is_reserved_key(key) {
        return Err(HotkeySettingsError::ReservedKey {
            key: key.to_string(),
        });
    }

    // Check if the key is a modifier key used alone
    let key_is_modifier = is_modifier_key(key);
    if key_is_modifier && !allow_double_tap {
        return Err(HotkeySettingsError::SingleModifier);
    }

    // Non-modifier keys require at least one modifier
    if !key_is_modifier && modifiers.is_empty() {
        return Err(HotkeySettingsError::InvalidBinding {
            reason: "at least one modifier (Command, Option, Control, Shift) is required"
                .to_string(),
        });
    }

    // Validate modifier names
    for modifier in modifiers {
        if !is_valid_modifier(modifier) {
            return Err(HotkeySettingsError::InvalidBinding {
                reason: format!(
                    "'{}' is not a valid modifier. Use Command, Option, Control, or Shift.",
                    modifier
                ),
            });
        }
    }

    Ok(())
}

/// Checks if a key name represents a modifier key.
#[must_use]
pub fn is_modifier_key(key: &str) -> bool {
    let key_lower = key.to_lowercase();
    matches!(
        key_lower.as_str(),
        "command" | "cmd" | "option" | "alt" | "control" | "ctrl" | "shift"
    )
}

/// Checks if a modifier name is valid.
#[must_use]
pub fn is_valid_modifier(modifier: &str) -> bool {
    let modifier_lower = modifier.to_lowercase();
    matches!(
        modifier_lower.as_str(),
        "command" | "cmd" | "option" | "alt" | "control" | "ctrl" | "shift"
    )
}

/// Parses a modifier string into a `Modifier` enum value.
#[must_use]
pub fn parse_modifier(modifier: &str) -> Option<Modifier> {
    let modifier_lower = modifier.to_lowercase();
    match modifier_lower.as_str() {
        "command" | "cmd" => Some(Modifier::Command),
        "option" | "alt" => Some(Modifier::Option),
        "control" | "ctrl" => Some(Modifier::Control),
        "shift" => Some(Modifier::Shift),
        _ => None,
    }
}

/// Converts a list of modifier strings to a `Modifiers` struct.
#[must_use]
pub fn parse_modifiers(modifier_strs: &[String]) -> Modifiers {
    let mut modifiers = Modifiers::default();
    for modifier_str in modifier_strs {
        if let Some(modifier) = parse_modifier(modifier_str) {
            match modifier {
                Modifier::Command => modifiers.command = true,
                Modifier::Option => modifiers.option = true,
                Modifier::Control => modifiers.control = true,
                Modifier::Shift => modifiers.shift = true,
            }
        }
    }
    modifiers
}

// =============================================================================
// Key Capture State
// =============================================================================

/// State for hotkey key capture mode.
///
/// When in capture mode, the next key press (with modifiers) will be captured
/// and used as the new hotkey binding.
#[derive(Debug, Clone, Default)]
pub struct KeyCaptureState {
    /// Whether capture mode is active.
    pub is_capturing: bool,

    /// The captured key (if any).
    pub captured_key: Option<String>,

    /// The captured modifiers.
    pub captured_modifiers: Vec<String>,

    /// Whether the capture is complete and valid.
    pub is_complete: bool,

    /// Error message if the capture is invalid.
    pub error: Option<String>,
}

impl KeyCaptureState {
    /// Creates a new key capture state (not capturing).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_capturing: false,
            captured_key: None,
            captured_modifiers: Vec::new(),
            is_complete: false,
            error: None,
        }
    }

    /// Starts capture mode.
    pub fn start_capture(&mut self) {
        self.is_capturing = true;
        self.captured_key = None;
        self.captured_modifiers = Vec::new();
        self.is_complete = false;
        self.error = None;
        debug!("Started hotkey capture mode");
    }

    /// Cancels capture mode.
    pub fn cancel_capture(&mut self) {
        self.is_capturing = false;
        self.captured_key = None;
        self.captured_modifiers = Vec::new();
        self.is_complete = false;
        self.error = None;
        debug!("Cancelled hotkey capture mode");
    }

    /// Records a key press during capture.
    ///
    /// Returns `true` if the capture is now complete (valid key was pressed).
    pub fn on_key_press(&mut self, key: &str, modifiers: &[String]) -> bool {
        if !self.is_capturing {
            return false;
        }

        // Skip if only modifier keys are pressed
        if is_modifier_key(key) && modifiers.is_empty() {
            debug!("Ignoring lone modifier key press: {}", key);
            return false;
        }

        // Validate the binding
        match validate_binding(key, modifiers, false) {
            Ok(()) => {
                self.captured_key = Some(key.to_string());
                self.captured_modifiers = modifiers.to_vec();
                self.is_complete = true;
                self.error = None;
                debug!("Captured hotkey: {} + {:?}", key, self.captured_modifiers);
                true
            },
            Err(err) => {
                self.error = Some(err.user_message());
                debug!("Invalid capture: {}", err);
                false
            },
        }
    }

    /// Gets the captured binding if complete.
    #[must_use]
    pub fn get_captured_binding(&self) -> Option<(&str, &[String])> {
        if self.is_complete {
            self.captured_key
                .as_deref()
                .map(|key| (key, self.captured_modifiers.as_slice()))
        } else {
            None
        }
    }

    /// Returns whether capture is currently in progress.
    #[must_use]
    pub const fn is_capturing(&self) -> bool {
        self.is_capturing
    }
}

impl fmt::Display for KeyCaptureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_capturing {
            if let Some(ref key) = self.captured_key {
                let mods = if self.captured_modifiers.is_empty() {
                    String::new()
                } else {
                    format!("{} + ", self.captured_modifiers.join(" + "))
                };
                write!(f, "{}{}", mods, key)
            } else {
                write!(f, "Press a key combination...")
            }
        } else {
            write!(f, "Not capturing")
        }
    }
}

// =============================================================================
// Hotkey Settings
// =============================================================================

/// Hotkey settings management.
///
/// This struct manages the hotkey configuration, including:
/// - Current binding
/// - Key capture mode
/// - Validation
/// - Persistence
#[derive(Debug)]
pub struct HotkeySettings {
    /// The current hotkey configuration.
    current_config: HotkeyConfig,

    /// Key capture state.
    capture_state: KeyCaptureState,

    /// Whether double-tap mode is enabled.
    double_tap_enabled: bool,
}

impl HotkeySettings {
    /// Creates new hotkey settings from a configuration.
    #[must_use]
    pub fn new(config: HotkeyConfig) -> Self {
        let double_tap_enabled = config.double_tap_modifier.is_some();
        Self {
            current_config: config,
            capture_state: KeyCaptureState::new(),
            double_tap_enabled,
        }
    }

    /// Creates hotkey settings with the default binding.
    #[must_use]
    pub fn with_default() -> Self {
        Self::new(HotkeyConfig::default())
    }

    /// Returns the current hotkey configuration.
    #[must_use]
    pub fn current_config(&self) -> &HotkeyConfig {
        &self.current_config
    }

    /// Returns a mutable reference to the capture state.
    pub fn capture_state_mut(&mut self) -> &mut KeyCaptureState {
        &mut self.capture_state
    }

    /// Returns the capture state.
    #[must_use]
    pub fn capture_state(&self) -> &KeyCaptureState {
        &self.capture_state
    }

    /// Starts key capture mode.
    pub fn start_capture(&mut self) {
        self.capture_state.start_capture();
    }

    /// Cancels key capture mode.
    pub fn cancel_capture(&mut self) {
        self.capture_state.cancel_capture();
    }

    /// Handles a key press during capture.
    pub fn on_key_press(&mut self, key: &str, modifiers: &[String]) -> bool {
        self.capture_state.on_key_press(key, modifiers)
    }

    /// Applies the captured binding as the new hotkey.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No binding has been captured
    /// - The binding is invalid
    /// - The binding conflicts with another app
    pub fn apply_captured_binding(&mut self) -> Result<HotkeyConfig, HotkeySettingsError> {
        let (key, modifiers) = self.capture_state.get_captured_binding().ok_or_else(|| {
            HotkeySettingsError::InvalidBinding {
                reason: "no binding captured".to_string(),
            }
        })?;

        // Create new config
        let new_config = HotkeyConfig {
            key: key.to_string(),
            modifiers: modifiers.to_vec(),
            double_tap_modifier: self.current_config.double_tap_modifier.clone(),
        };

        // Validate
        validate_binding(
            &new_config.key,
            &new_config.modifiers,
            self.double_tap_enabled,
        )?;

        // Check for conflicts
        let binding = config_to_binding(&new_config);
        if let Some(conflict) = detect_hotkey_conflict(&binding) {
            warn!(
                "Detected hotkey conflict with {} for binding {:?}",
                conflict.app_name, binding
            );
            return Err(HotkeySettingsError::RegistrationFailed(
                conflict.into_error(),
            ));
        }

        // Apply the new config
        self.current_config = new_config.clone();
        self.capture_state.cancel_capture();

        info!(
            "Applied new hotkey binding: {} + {:?}",
            self.current_config.key, self.current_config.modifiers
        );

        Ok(new_config)
    }

    /// Sets the double-tap modifier.
    pub fn set_double_tap_modifier(&mut self, modifier: Option<String>) {
        self.current_config.double_tap_modifier = modifier.clone();
        self.double_tap_enabled = modifier.is_some();
        debug!(
            "Set double-tap modifier: {:?}",
            self.current_config.double_tap_modifier
        );
    }

    /// Returns whether double-tap mode is enabled.
    #[must_use]
    pub const fn is_double_tap_enabled(&self) -> bool {
        self.double_tap_enabled
    }

    /// Converts the current config to a display string.
    #[must_use]
    pub fn display_binding(&self) -> String {
        format_binding(&self.current_config.key, &self.current_config.modifiers)
    }
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self::with_default()
    }
}

// =============================================================================
// Hotkey Change Manager
// =============================================================================

/// Manages hotkey changes with proper re-registration.
///
/// This struct handles the complete workflow of changing a hotkey:
/// 1. Unregister the old hotkey
/// 2. Validate the new binding
/// 3. Register the new hotkey
/// 4. Persist to config file
pub struct HotkeyChangeManager<'a> {
    /// Reference to the hotkey manager.
    hotkey_manager: &'a mut HotkeyManager,

    /// Hotkey settings.
    settings: &'a mut HotkeySettings,
}

impl<'a> HotkeyChangeManager<'a> {
    /// Creates a new hotkey change manager.
    pub fn new(hotkey_manager: &'a mut HotkeyManager, settings: &'a mut HotkeySettings) -> Self {
        Self {
            hotkey_manager,
            settings,
        }
    }

    /// Changes the hotkey to a new binding.
    ///
    /// This performs the complete change workflow:
    /// 1. Validate the new binding
    /// 2. Unregister the old hotkey
    /// 3. Register the new hotkey
    /// 4. Update the settings
    ///
    /// # Errors
    ///
    /// Returns an error if validation or registration fails.
    /// If registration fails, attempts to restore the old binding.
    pub fn change_hotkey(
        &mut self,
        new_key: &str,
        new_modifiers: &[String],
    ) -> Result<HotkeyConfig, HotkeySettingsError> {
        // Store old binding for rollback
        let old_config = self.settings.current_config.clone();

        // Validate new binding
        validate_binding(new_key, new_modifiers, self.settings.double_tap_enabled)?;

        // Create new config
        let new_config = HotkeyConfig {
            key: new_key.to_string(),
            modifiers: new_modifiers.to_vec(),
            double_tap_modifier: old_config.double_tap_modifier.clone(),
        };

        // Convert to binding
        let new_binding = config_to_binding(&new_config);

        // Check for conflicts
        if let Some(conflict) = detect_hotkey_conflict(&new_binding) {
            warn!(
                "Detected hotkey conflict with {} for new binding {:?}",
                conflict.app_name, new_binding
            );
            return Err(HotkeySettingsError::RegistrationFailed(
                conflict.into_error(),
            ));
        }

        // Unregister old hotkey
        debug!("Unregistering old hotkey");
        self.hotkey_manager.unregister();

        // Try to register new hotkey
        debug!("Registering new hotkey: {:?}", new_binding);
        if let Err(err) = self.hotkey_manager.register(new_binding) {
            // Rollback: try to re-register old binding
            error!(
                "Failed to register new hotkey: {}. Attempting rollback.",
                err
            );
            let old_binding = config_to_binding(&old_config);
            if let Err(rollback_err) = self.hotkey_manager.register(old_binding) {
                error!(
                    "Rollback failed! Could not re-register old hotkey: {}",
                    rollback_err
                );
            } else {
                debug!("Successfully rolled back to old hotkey");
            }
            return Err(HotkeySettingsError::RegistrationFailed(err));
        }

        // Update settings
        self.settings.current_config = new_config.clone();

        info!(
            "Successfully changed hotkey to {} + {:?}",
            new_key, new_modifiers
        );

        Ok(new_config)
    }

    /// Changes the hotkey using a captured binding from the settings.
    ///
    /// # Errors
    ///
    /// Returns an error if no binding was captured or if the change fails.
    pub fn apply_captured_change(&mut self) -> Result<HotkeyConfig, HotkeySettingsError> {
        let (key, modifiers) = self
            .settings
            .capture_state
            .get_captured_binding()
            .ok_or_else(|| HotkeySettingsError::InvalidBinding {
                reason: "no binding captured".to_string(),
            })?;

        let key = key.to_string();
        let modifiers: Vec<String> = modifiers.to_vec();

        self.change_hotkey(&key, &modifiers)
    }
}

// =============================================================================
// Config Persistence
// =============================================================================

/// Saves the hotkey configuration to the app config file.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn save_hotkey_config(
    config_path: &Path,
    hotkey_config: &HotkeyConfig,
) -> Result<(), HotkeySettingsError> {
    // Load existing config or create default
    let mut config = load_config(config_path).unwrap_or_default();

    // Update hotkey section
    config.hotkey = hotkey_config.clone();

    // Serialize to TOML
    let toml_str =
        toml::to_string_pretty(&config).map_err(|e| HotkeySettingsError::SaveFailed {
            reason: format!("failed to serialize config: {}", e),
        })?;

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HotkeySettingsError::SaveFailed {
            reason: format!("failed to create config directory: {}", e),
        })?;
    }

    // Write atomically using a temp file
    let temp_path = config_path.with_extension("toml.tmp");
    std::fs::write(&temp_path, &toml_str).map_err(|e| HotkeySettingsError::SaveFailed {
        reason: format!("failed to write temp config file: {}", e),
    })?;

    std::fs::rename(&temp_path, config_path).map_err(|e| HotkeySettingsError::SaveFailed {
        reason: format!("failed to rename temp config file: {}", e),
    })?;

    debug!("Saved hotkey config to {:?}", config_path);
    Ok(())
}

/// Loads the app configuration from a file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_config(config_path: &Path) -> Result<Config, HotkeySettingsError> {
    let content =
        std::fs::read_to_string(config_path).map_err(|e| HotkeySettingsError::LoadFailed {
            reason: format!("failed to read config file: {}", e),
        })?;

    let config: Config = toml::from_str(&content).map_err(|e| HotkeySettingsError::LoadFailed {
        reason: format!("failed to parse config file: {}", e),
    })?;

    debug!("Loaded config from {:?}", config_path);
    Ok(config)
}

/// Returns the default config file path.
#[must_use]
pub fn default_config_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config")
        .join("photoncast")
        .join("config.toml")
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Converts a `HotkeyConfig` to a `HotkeyBinding`.
#[must_use]
pub fn config_to_binding(config: &HotkeyConfig) -> HotkeyBinding {
    HotkeyBinding::new(&config.key, parse_modifiers(&config.modifiers))
}

/// Formats a binding for display.
#[must_use]
pub fn format_binding(key: &str, modifiers: &[String]) -> String {
    let mut parts = Vec::new();

    for modifier in modifiers {
        let symbol = match modifier.to_lowercase().as_str() {
            "command" | "cmd" => "⌘",
            "option" | "alt" => "⌥",
            "control" | "ctrl" => "⌃",
            "shift" => "⇧",
            _ => modifier.as_str(),
        };
        parts.push(symbol.to_string());
    }

    parts.push(key.to_string());
    parts.join("")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Reserved Key Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_reserved_keys() {
        assert!(is_reserved_key("Power"));
        assert!(is_reserved_key("power")); // Case insensitive
        assert!(is_reserved_key("Eject"));
        assert!(is_reserved_key("CapsLock"));
        assert!(is_reserved_key("F1"));
        assert!(is_reserved_key("F12"));

        assert!(!is_reserved_key("Space"));
        assert!(!is_reserved_key("A"));
        assert!(!is_reserved_key("F5"));
        assert!(!is_reserved_key("Return"));
    }

    // -------------------------------------------------------------------------
    // Modifier Detection Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_modifier_key() {
        assert!(is_modifier_key("Command"));
        assert!(is_modifier_key("command"));
        assert!(is_modifier_key("Cmd"));
        assert!(is_modifier_key("Option"));
        assert!(is_modifier_key("Alt"));
        assert!(is_modifier_key("Control"));
        assert!(is_modifier_key("Ctrl"));
        assert!(is_modifier_key("Shift"));

        assert!(!is_modifier_key("Space"));
        assert!(!is_modifier_key("A"));
        assert!(!is_modifier_key("Return"));
    }

    #[test]
    fn test_is_valid_modifier() {
        assert!(is_valid_modifier("Command"));
        assert!(is_valid_modifier("Cmd"));
        assert!(is_valid_modifier("Option"));
        assert!(is_valid_modifier("Alt"));
        assert!(is_valid_modifier("Control"));
        assert!(is_valid_modifier("Ctrl"));
        assert!(is_valid_modifier("Shift"));

        assert!(!is_valid_modifier("Space"));
        assert!(!is_valid_modifier("Super"));
        assert!(!is_valid_modifier("Meta"));
    }

    #[test]
    fn test_parse_modifier() {
        assert_eq!(parse_modifier("Command"), Some(Modifier::Command));
        assert_eq!(parse_modifier("Cmd"), Some(Modifier::Command));
        assert_eq!(parse_modifier("Option"), Some(Modifier::Option));
        assert_eq!(parse_modifier("Alt"), Some(Modifier::Option));
        assert_eq!(parse_modifier("Control"), Some(Modifier::Control));
        assert_eq!(parse_modifier("Ctrl"), Some(Modifier::Control));
        assert_eq!(parse_modifier("Shift"), Some(Modifier::Shift));

        assert_eq!(parse_modifier("Space"), None);
        assert_eq!(parse_modifier("Invalid"), None);
    }

    #[test]
    fn test_parse_modifiers() {
        let modifiers = parse_modifiers(&["Command".to_string(), "Shift".to_string()]);
        assert!(modifiers.command);
        assert!(modifiers.shift);
        assert!(!modifiers.option);
        assert!(!modifiers.control);

        let modifiers = parse_modifiers(&["Ctrl".to_string(), "Alt".to_string()]);
        assert!(modifiers.control);
        assert!(modifiers.option);
        assert!(!modifiers.command);
        assert!(!modifiers.shift);
    }

    // -------------------------------------------------------------------------
    // Binding Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_binding_valid() {
        // Valid: Cmd+Space
        assert!(validate_binding("Space", &["Command".to_string()], false).is_ok());

        // Valid: Cmd+Shift+K
        assert!(
            validate_binding("K", &["Command".to_string(), "Shift".to_string()], false).is_ok()
        );

        // Valid: Ctrl+Option+Return
        assert!(validate_binding(
            "Return",
            &["Control".to_string(), "Option".to_string()],
            false
        )
        .is_ok());
    }

    #[test]
    fn test_validate_binding_empty_key() {
        let result = validate_binding("", &["Command".to_string()], false);
        assert!(matches!(
            result,
            Err(HotkeySettingsError::InvalidBinding { .. })
        ));
    }

    #[test]
    fn test_validate_binding_reserved_key() {
        let result = validate_binding("Power", &["Command".to_string()], false);
        assert!(matches!(
            result,
            Err(HotkeySettingsError::ReservedKey { .. })
        ));
    }

    #[test]
    fn test_validate_binding_no_modifiers() {
        // Single key without modifiers is not allowed
        let result = validate_binding("Space", &[], false);
        assert!(matches!(
            result,
            Err(HotkeySettingsError::InvalidBinding { .. })
        ));
    }

    #[test]
    fn test_validate_binding_single_modifier() {
        // Single modifier as key without double-tap is not allowed
        let result = validate_binding("Command", &[], false);
        assert!(matches!(result, Err(HotkeySettingsError::SingleModifier)));

        // But with double-tap mode, it's allowed
        assert!(validate_binding("Command", &[], true).is_ok());
    }

    #[test]
    fn test_validate_binding_invalid_modifier() {
        let result = validate_binding("Space", &["Super".to_string()], false);
        assert!(matches!(
            result,
            Err(HotkeySettingsError::InvalidBinding { .. })
        ));
    }

    // -------------------------------------------------------------------------
    // KeyCaptureState Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_key_capture_state_new() {
        let state = KeyCaptureState::new();
        assert!(!state.is_capturing);
        assert!(state.captured_key.is_none());
        assert!(state.captured_modifiers.is_empty());
        assert!(!state.is_complete);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_key_capture_start() {
        let mut state = KeyCaptureState::new();
        state.start_capture();

        assert!(state.is_capturing);
        assert!(state.captured_key.is_none());
        assert!(!state.is_complete);
    }

    #[test]
    fn test_key_capture_cancel() {
        let mut state = KeyCaptureState::new();
        state.start_capture();
        state.captured_key = Some("A".to_string());
        state.cancel_capture();

        assert!(!state.is_capturing);
        assert!(state.captured_key.is_none());
        assert!(!state.is_complete);
    }

    #[test]
    fn test_key_capture_valid_press() {
        let mut state = KeyCaptureState::new();
        state.start_capture();

        let result = state.on_key_press("K", &["Command".to_string()]);

        assert!(result);
        assert!(state.is_complete);
        assert_eq!(state.captured_key, Some("K".to_string()));
        assert_eq!(state.captured_modifiers, vec!["Command".to_string()]);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_key_capture_lone_modifier() {
        let mut state = KeyCaptureState::new();
        state.start_capture();

        // Pressing only a modifier should be ignored
        let result = state.on_key_press("Command", &[]);

        assert!(!result);
        assert!(!state.is_complete);
        assert!(state.captured_key.is_none());
    }

    #[test]
    fn test_key_capture_reserved_key() {
        let mut state = KeyCaptureState::new();
        state.start_capture();

        let result = state.on_key_press("Power", &["Command".to_string()]);

        assert!(!result);
        assert!(!state.is_complete);
        assert!(state.error.is_some());
    }

    #[test]
    fn test_key_capture_not_capturing() {
        let mut state = KeyCaptureState::new();
        // Not in capture mode
        let result = state.on_key_press("K", &["Command".to_string()]);
        assert!(!result);
    }

    #[test]
    fn test_key_capture_get_binding() {
        let mut state = KeyCaptureState::new();
        state.start_capture();
        state.on_key_press("Space", &["Command".to_string(), "Shift".to_string()]);

        let binding = state.get_captured_binding();
        assert!(binding.is_some());
        let (key, modifiers) = binding.unwrap();
        assert_eq!(key, "Space");
        assert_eq!(modifiers, &["Command".to_string(), "Shift".to_string()]);
    }

    #[test]
    fn test_key_capture_display() {
        let mut state = KeyCaptureState::new();
        assert_eq!(format!("{}", state), "Not capturing");

        state.start_capture();
        assert_eq!(format!("{}", state), "Press a key combination...");

        state.on_key_press("K", &["Command".to_string()]);
        assert_eq!(format!("{}", state), "Command + K");
    }

    // -------------------------------------------------------------------------
    // HotkeySettings Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_hotkey_settings_new() {
        let config = HotkeyConfig::default();
        let settings = HotkeySettings::new(config);

        assert_eq!(settings.current_config().key, "Space");
        assert_eq!(settings.current_config().modifiers, vec!["Command"]);
        assert!(!settings.is_double_tap_enabled());
    }

    #[test]
    fn test_hotkey_settings_with_default() {
        let settings = HotkeySettings::with_default();
        assert_eq!(settings.current_config().key, "Space");
    }

    #[test]
    fn test_hotkey_settings_display_binding() {
        let settings = HotkeySettings::with_default();
        assert_eq!(settings.display_binding(), "⌘Space");

        let config = HotkeyConfig {
            key: "K".to_string(),
            modifiers: vec!["Command".to_string(), "Shift".to_string()],
            double_tap_modifier: None,
        };
        let settings = HotkeySettings::new(config);
        assert_eq!(settings.display_binding(), "⌘⇧K");
    }

    #[test]
    fn test_hotkey_settings_double_tap() {
        let config = HotkeyConfig {
            key: "Space".to_string(),
            modifiers: vec!["Command".to_string()],
            double_tap_modifier: Some("Command".to_string()),
        };
        let settings = HotkeySettings::new(config);
        assert!(settings.is_double_tap_enabled());
    }

    #[test]
    fn test_hotkey_settings_capture_flow() {
        let mut settings = HotkeySettings::with_default();

        // Start capture
        settings.start_capture();
        assert!(settings.capture_state().is_capturing);

        // Press a key
        let captured = settings.on_key_press("K", &["Command".to_string(), "Shift".to_string()]);
        assert!(captured);

        // Apply captured binding
        let result = settings.apply_captured_binding();
        assert!(result.is_ok());

        let new_config = result.unwrap();
        assert_eq!(new_config.key, "K");
        assert_eq!(
            new_config.modifiers,
            vec!["Command".to_string(), "Shift".to_string()]
        );
    }

    // -------------------------------------------------------------------------
    // Format Binding Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_format_binding() {
        assert_eq!(format_binding("Space", &["Command".to_string()]), "⌘Space");
        assert_eq!(
            format_binding("K", &["Command".to_string(), "Shift".to_string()]),
            "⌘⇧K"
        );
        assert_eq!(
            format_binding("A", &["Control".to_string(), "Option".to_string()]),
            "⌃⌥A"
        );
        assert_eq!(format_binding("Tab", &["Cmd".to_string()]), "⌘Tab");
    }

    // -------------------------------------------------------------------------
    // Config Conversion Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_config_to_binding() {
        let config = HotkeyConfig {
            key: "Space".to_string(),
            modifiers: vec!["Command".to_string()],
            double_tap_modifier: None,
        };

        let binding = config_to_binding(&config);
        assert_eq!(binding.key, "Space");
        assert!(binding.modifiers.command);
        assert!(!binding.modifiers.option);
        assert!(!binding.modifiers.control);
        assert!(!binding.modifiers.shift);
    }

    // -------------------------------------------------------------------------
    // Error Message Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_error_messages() {
        let err = HotkeySettingsError::InvalidBinding {
            reason: "test".to_string(),
        };
        assert!(err.user_message().contains("not valid"));

        let err = HotkeySettingsError::ReservedKey {
            key: "Power".to_string(),
        };
        assert!(err.user_message().contains("Power"));

        let err = HotkeySettingsError::SingleModifier;
        assert!(err.user_message().contains("modifier key alone"));
    }
}
