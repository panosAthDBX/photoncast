//! Global hotkey registration, double-tap modifier detection, and conflict detection.

use std::path::Path;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, warn};

/// Errors that can occur during hotkey registration.
#[derive(Error, Debug, Clone)]
pub enum HotkeyError {
    /// Accessibility permission is required but not granted.
    #[error("accessibility permission required")]
    PermissionDenied,

    /// The hotkey conflicts with another application.
    #[error("hotkey conflict with '{app}': {suggestion}")]
    ConflictDetected {
        /// The conflicting application name.
        app: String,
        /// User-friendly suggestion for resolving the conflict.
        suggestion: String,
    },

    /// Failed to register the hotkey.
    #[error("failed to register hotkey: {reason}")]
    RegistrationFailed {
        /// The reason for failure.
        reason: String,
    },

    /// The key binding is invalid.
    #[error("invalid key binding")]
    InvalidBinding,
}

impl HotkeyError {
    /// Returns a user-friendly message explaining the error.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::PermissionDenied => {
                "PhotonCast needs accessibility permission to register global hotkeys. \
                 Please grant access in System Settings → Privacy & Security → Accessibility."
                    .to_string()
            },
            Self::ConflictDetected { app, suggestion } => {
                format!("The hotkey you selected conflicts with {app}. {suggestion}")
            },
            Self::RegistrationFailed { reason } => {
                format!("Failed to register hotkey: {reason}. Try a different key combination.")
            },
            Self::InvalidBinding => {
                "The selected key combination is not valid. Please choose a different hotkey."
                    .to_string()
            },
        }
    }

    /// Returns true if this error can be recovered from by user action.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::PermissionDenied | Self::ConflictDetected { .. } | Self::InvalidBinding
        )
    }
}

/// Information about a detected hotkey conflict.
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// The name of the application that has the conflicting hotkey.
    pub app_name: String,
    /// The conflicting hotkey binding.
    pub hotkey: HotkeyBinding,
    /// User-friendly suggestion for resolving the conflict.
    pub suggestion: String,
}

impl ConflictInfo {
    /// Creates a new conflict info for Spotlight.
    #[must_use]
    pub fn spotlight(hotkey: HotkeyBinding) -> Self {
        Self {
            app_name: "Spotlight".to_string(),
            hotkey,
            suggestion: "Disable Spotlight shortcut in System Settings → Keyboard → \
                         Keyboard Shortcuts → Spotlight, or choose a different hotkey for PhotonCast."
                .to_string(),
        }
    }

    /// Creates a conflict info for a generic application.
    #[must_use]
    pub fn generic(app_name: impl Into<String>, hotkey: HotkeyBinding) -> Self {
        let app = app_name.into();
        Self {
            suggestion: format!(
                "Disable the hotkey in {app}'s settings, or choose a different hotkey for PhotonCast."
            ),
            app_name: app,
            hotkey,
        }
    }

    /// Converts this conflict info into a `HotkeyError::ConflictDetected`.
    #[must_use]
    pub fn into_error(self) -> HotkeyError {
        HotkeyError::ConflictDetected {
            app: self.app_name,
            suggestion: self.suggestion,
        }
    }
}

/// A single modifier key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// Command (⌘) key.
    Command,
    /// Option (⌥) key.
    Option,
    /// Control (⌃) key.
    Control,
    /// Shift (⇧) key.
    Shift,
}

impl std::fmt::Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command => write!(f, "Command"),
            Self::Option => write!(f, "Option"),
            Self::Control => write!(f, "Control"),
            Self::Shift => write!(f, "Shift"),
        }
    }
}

/// Key modifiers for hotkey bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Command key.
    pub command: bool,
    /// Option/Alt key.
    pub option: bool,
    /// Control key.
    pub control: bool,
    /// Shift key.
    pub shift: bool,
}

impl Modifiers {
    /// Command modifier only.
    pub const COMMAND: Self = Self {
        command: true,
        option: false,
        control: false,
        shift: false,
    };

    /// Option modifier only.
    pub const OPTION: Self = Self {
        command: false,
        option: true,
        control: false,
        shift: false,
    };

    /// Control modifier only.
    pub const CONTROL: Self = Self {
        command: false,
        option: false,
        control: true,
        shift: false,
    };

    /// Shift modifier only.
    pub const SHIFT: Self = Self {
        command: false,
        option: false,
        control: false,
        shift: true,
    };
}

/// A hotkey binding (key + modifiers).
#[derive(Debug, Clone)]
pub struct HotkeyBinding {
    /// The key code.
    pub key: String,
    /// The modifiers.
    pub modifiers: Modifiers,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            key: "Space".to_string(),
            modifiers: Modifiers::COMMAND,
        }
    }
}

impl HotkeyBinding {
    /// Creates a new hotkey binding.
    #[must_use]
    pub fn new(key: impl Into<String>, modifiers: Modifiers) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    /// Checks if this binding matches the default Spotlight shortcut (Cmd+Space).
    #[must_use]
    pub fn is_spotlight_default(&self) -> bool {
        self.key == "Space"
            && self.modifiers.command
            && !self.modifiers.option
            && !self.modifiers.control
            && !self.modifiers.shift
    }
}

/// Default double-tap detection threshold in milliseconds.
pub const DEFAULT_DOUBLE_TAP_THRESHOLD_MS: u64 = 300;

/// Detects double-tap of a modifier key.
///
/// This detector tracks the timing of modifier key presses to detect
/// when a user quickly taps the same modifier key twice within a threshold.
///
/// # Example
///
/// ```
/// use photoncast_core::platform::hotkey::{DoubleTapDetector, Modifier};
///
/// let mut detector = DoubleTapDetector::new(Modifier::Command, 300);
///
/// // First press - no detection yet
/// assert!(!detector.on_modifier_event(Modifier::Command, true));
///
/// // Release
/// detector.on_modifier_event(Modifier::Command, false);
///
/// // Second press within threshold - double-tap detected!
/// // (In practice, you'd need to control timing for this to trigger)
/// ```
#[derive(Debug)]
pub struct DoubleTapDetector {
    /// The modifier key to detect double-taps for.
    target_modifier: Modifier,
    /// The maximum time between taps to count as a double-tap.
    threshold: Duration,
    /// The timestamp of the last press of the target modifier.
    last_press: Option<Instant>,
}

impl DoubleTapDetector {
    /// Creates a new double-tap detector for the specified modifier.
    ///
    /// # Arguments
    ///
    /// * `modifier` - The modifier key to detect double-taps for.
    /// * `threshold_ms` - The maximum time between taps in milliseconds.
    #[must_use]
    pub fn new(modifier: Modifier, threshold_ms: u64) -> Self {
        Self {
            target_modifier: modifier,
            threshold: Duration::from_millis(threshold_ms),
            last_press: None,
        }
    }

    /// Creates a new double-tap detector with the default threshold (300ms).
    #[must_use]
    pub fn with_default_threshold(modifier: Modifier) -> Self {
        Self::new(modifier, DEFAULT_DOUBLE_TAP_THRESHOLD_MS)
    }

    /// Handles a modifier key event and returns whether a double-tap was detected.
    ///
    /// # Arguments
    ///
    /// * `modifier` - The modifier key that was pressed or released.
    /// * `pressed` - `true` if the key was pressed, `false` if released.
    ///
    /// # Returns
    ///
    /// `true` if a double-tap was detected, `false` otherwise.
    /// A double-tap is detected when the target modifier is pressed twice
    /// within the threshold duration.
    pub fn on_modifier_event(&mut self, modifier: Modifier, pressed: bool) -> bool {
        // Only track presses of our target modifier
        if modifier != self.target_modifier || !pressed {
            return false;
        }

        let now = Instant::now();

        if let Some(last) = self.last_press {
            if now.duration_since(last) <= self.threshold {
                // Double-tap detected! Reset state.
                self.last_press = None;
                return true;
            }
        }

        // Record this press for potential double-tap
        self.last_press = Some(now);
        false
    }

    /// Resets the detector state.
    ///
    /// Call this after a double-tap is handled or when you want to
    /// cancel any pending double-tap detection.
    pub fn reset(&mut self) {
        self.last_press = None;
    }

    /// Returns the target modifier key.
    #[must_use]
    pub const fn target_modifier(&self) -> Modifier {
        self.target_modifier
    }

    /// Returns the double-tap threshold duration.
    #[must_use]
    pub const fn threshold(&self) -> Duration {
        self.threshold
    }

    /// Sets a new threshold duration.
    pub fn set_threshold(&mut self, threshold_ms: u64) {
        self.threshold = Duration::from_millis(threshold_ms);
    }

    /// Returns whether there is a pending first tap that hasn't timed out yet.
    ///
    /// This can be useful for UI feedback (e.g., showing a visual indicator
    /// after the first tap).
    #[must_use]
    pub fn has_pending_tap(&self) -> bool {
        if let Some(last) = self.last_press {
            Instant::now().duration_since(last) <= self.threshold
        } else {
            false
        }
    }
}

impl Clone for DoubleTapDetector {
    fn clone(&self) -> Self {
        Self {
            target_modifier: self.target_modifier,
            threshold: self.threshold,
            // Don't clone the timing state
            last_press: None,
        }
    }
}

/// Manages global hotkey registration.
#[derive(Debug, Default)]
pub struct HotkeyManager {
    /// The currently registered binding.
    current_binding: Option<HotkeyBinding>,
    /// Whether a hotkey is currently registered.
    is_registered: bool,
    /// Optional double-tap modifier detection.
    double_tap_detector: Option<DoubleTapDetector>,
}

impl HotkeyManager {
    /// Creates a new hotkey manager.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current_binding: None,
            is_registered: false,
            double_tap_detector: None,
        }
    }

    /// Registers a global hotkey.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Accessibility permission is not granted
    /// - The hotkey conflicts with another application (e.g., Spotlight)
    /// - Registration fails for technical reasons
    pub fn register(&mut self, binding: HotkeyBinding) -> Result<(), HotkeyError> {
        // Check accessibility permission
        if !crate::platform::accessibility::check_accessibility_permission() {
            return Err(HotkeyError::PermissionDenied);
        }

        // Check for conflicts and return detailed error if found
        if let Some(conflict) = detect_hotkey_conflict(&binding) {
            return Err(conflict.into_error());
        }

        // Unregister existing hotkey
        self.unregister();

        // STUB: CGEventTap registration not yet implemented.
        // This function currently only stores the binding without actually
        // registering it with the system. Global hotkey activation will not work
        // until CGEventTap integration is completed.
        // See: https://developer.apple.com/documentation/coregraphics/cgeventtap

        self.current_binding = Some(binding);
        self.is_registered = true;

        Ok(())
    }

    /// Registers a global hotkey, bypassing conflict detection.
    ///
    /// **WARNING: This is currently a stub.** See [`register`] for details.
    ///
    /// Use this with caution - it will register the hotkey even if it conflicts
    /// with system shortcuts like Spotlight. The user should have been warned
    /// about the conflict before calling this.
    ///
    /// # Errors
    ///
    /// Returns an error if accessibility permission is not granted or registration fails.
    pub fn register_force(&mut self, binding: HotkeyBinding) -> Result<(), HotkeyError> {
        // Check accessibility permission
        if !crate::platform::accessibility::check_accessibility_permission() {
            return Err(HotkeyError::PermissionDenied);
        }

        // Unregister existing hotkey
        self.unregister();

        // STUB: CGEventTap registration not yet implemented.
        // See register() for details.

        self.current_binding = Some(binding);
        self.is_registered = true;

        Ok(())
    }

    /// Unregisters the current hotkey.
    pub fn unregister(&mut self) {
        // TODO: Implement actual unregistration
        self.current_binding = None;
        self.is_registered = false;
    }

    /// Returns the currently registered binding.
    #[must_use]
    pub fn current_binding(&self) -> Option<&HotkeyBinding> {
        self.current_binding.as_ref()
    }

    /// Returns true if a hotkey is registered.
    #[must_use]
    pub const fn is_registered(&self) -> bool {
        self.is_registered
    }

    /// Sets the double-tap detector for this manager.
    pub fn set_double_tap_detector(&mut self, detector: Option<DoubleTapDetector>) {
        self.double_tap_detector = detector;
    }

    /// Returns a reference to the double-tap detector if set.
    #[must_use]
    pub fn double_tap_detector(&self) -> Option<&DoubleTapDetector> {
        self.double_tap_detector.as_ref()
    }

    /// Returns a mutable reference to the double-tap detector if set.
    pub fn double_tap_detector_mut(&mut self) -> Option<&mut DoubleTapDetector> {
        self.double_tap_detector.as_mut()
    }
}

// =============================================================================
// Conflict Detection
// =============================================================================

/// Detects if a hotkey binding conflicts with another application.
///
/// Currently checks:
/// - Spotlight (Cmd+Space) - the default macOS search shortcut
///
/// # Returns
///
/// `Some(ConflictInfo)` if a conflict is detected, `None` otherwise.
pub fn detect_hotkey_conflict(binding: &HotkeyBinding) -> Option<ConflictInfo> {
    // Check Spotlight (Cmd+Space)
    if is_spotlight_hotkey(binding) && is_spotlight_enabled() {
        debug!("Detected conflict with Spotlight for hotkey: {:?}", binding);
        return Some(ConflictInfo::spotlight(binding.clone()));
    }

    // Future: Check other known conflicts (Raycast, Alfred, etc.)

    None
}

/// Checks if the given binding matches the Spotlight default shortcut (Cmd+Space).
fn is_spotlight_hotkey(binding: &HotkeyBinding) -> bool {
    binding.is_spotlight_default()
}

/// Checks if the Spotlight keyboard shortcut is enabled.
///
/// Reads the system symbolic hotkeys plist to determine if Spotlight's
/// keyboard shortcut (key 64) is enabled.
pub fn is_spotlight_enabled() -> bool {
    is_spotlight_enabled_from_path(&get_symbolic_hotkeys_path())
}

/// Returns the path to the symbolic hotkeys plist file.
fn get_symbolic_hotkeys_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Preferences/com.apple.symbolichotkeys.plist")
}

/// Checks if Spotlight is enabled by reading from the specified plist path.
///
/// This is separated from `is_spotlight_enabled` to allow testing with mock data.
fn is_spotlight_enabled_from_path(path: &Path) -> bool {
    read_spotlight_enabled_status(path).unwrap_or_else(|e| {
        warn!(
            "Failed to read Spotlight shortcut status from {:?}: {}. Assuming enabled.",
            path, e
        );
        // Default to true (assume enabled) if we can't read the plist
        // This is the safer assumption to prevent conflicts
        true
    })
}

/// Reads the Spotlight shortcut enabled status from the symbolic hotkeys plist.
///
/// The plist structure is:
/// ```xml
/// <dict>
///   <key>AppleSymbolicHotKeys</key>
///   <dict>
///     <key>64</key>
///     <dict>
///       <key>enabled</key>
///       <true/> or <false/>
///       ...
///     </dict>
///   </dict>
/// </dict>
/// ```
///
/// Key 64 is the Spotlight shortcut (Cmd+Space by default).
fn read_spotlight_enabled_status(path: &Path) -> Result<bool, String> {
    // Read and parse the plist file
    let plist_value: plist::Value =
        plist::from_file(path).map_err(|e| format!("Failed to parse plist: {e}"))?;

    // Navigate to AppleSymbolicHotKeys -> 64 -> enabled
    let dict = plist_value
        .as_dictionary()
        .ok_or("Plist root is not a dictionary")?;

    let hotkeys = dict
        .get("AppleSymbolicHotKeys")
        .ok_or("AppleSymbolicHotKeys key not found")?
        .as_dictionary()
        .ok_or("AppleSymbolicHotKeys is not a dictionary")?;

    let spotlight_entry = hotkeys
        .get("64")
        .ok_or("Spotlight hotkey (key 64) not found")?
        .as_dictionary()
        .ok_or("Spotlight entry is not a dictionary")?;

    let enabled = spotlight_entry
        .get("enabled")
        .ok_or("enabled key not found in Spotlight entry")?
        .as_boolean()
        .ok_or("enabled value is not a boolean")?;

    debug!(
        "Read Spotlight shortcut enabled status from {:?}: {}",
        path, enabled
    );

    Ok(enabled)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // -------------------------------------------------------------------------
    // HotkeyBinding Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_default_hotkey_binding() {
        let binding = HotkeyBinding::default();
        assert_eq!(binding.key, "Space");
        assert!(binding.modifiers.command);
        assert!(!binding.modifiers.option);
        assert!(!binding.modifiers.control);
        assert!(!binding.modifiers.shift);
    }

    #[test]
    fn test_is_spotlight_default() {
        // Default should match Spotlight
        let default = HotkeyBinding::default();
        assert!(default.is_spotlight_default());

        // Cmd+Space should match
        let cmd_space = HotkeyBinding::new("Space", Modifiers::COMMAND);
        assert!(cmd_space.is_spotlight_default());

        // Option+Space should not match
        let opt_space = HotkeyBinding::new("Space", Modifiers::OPTION);
        assert!(!opt_space.is_spotlight_default());

        // Cmd+A should not match
        let cmd_a = HotkeyBinding::new("A", Modifiers::COMMAND);
        assert!(!cmd_a.is_spotlight_default());

        // Cmd+Option+Space should not match
        let cmd_opt_space = HotkeyBinding::new(
            "Space",
            Modifiers {
                command: true,
                option: true,
                ..Default::default()
            },
        );
        assert!(!cmd_opt_space.is_spotlight_default());
    }

    // -------------------------------------------------------------------------
    // ConflictInfo Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_conflict_info_spotlight() {
        let binding = HotkeyBinding::default();
        let conflict = ConflictInfo::spotlight(binding);

        assert_eq!(conflict.app_name, "Spotlight");
        assert!(conflict.suggestion.contains("System Settings"));
        assert!(conflict.suggestion.contains("Keyboard Shortcuts"));
    }

    #[test]
    fn test_conflict_info_generic() {
        let binding = HotkeyBinding::new("A", Modifiers::COMMAND);
        let conflict = ConflictInfo::generic("TestApp", binding);

        assert_eq!(conflict.app_name, "TestApp");
        assert!(conflict.suggestion.contains("TestApp"));
    }

    #[test]
    fn test_conflict_info_into_error() {
        let binding = HotkeyBinding::default();
        let conflict = ConflictInfo::spotlight(binding);
        let error = conflict.into_error();

        match error {
            HotkeyError::ConflictDetected { app, suggestion } => {
                assert_eq!(app, "Spotlight");
                assert!(!suggestion.is_empty());
            },
            _ => panic!("Expected ConflictDetected error"),
        }
    }

    // -------------------------------------------------------------------------
    // HotkeyError Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_hotkey_error_user_message() {
        let error = HotkeyError::PermissionDenied;
        assert!(error.user_message().contains("accessibility"));

        let error = HotkeyError::ConflictDetected {
            app: "Spotlight".to_string(),
            suggestion: "Test suggestion".to_string(),
        };
        assert!(error.user_message().contains("Spotlight"));
        assert!(error.user_message().contains("Test suggestion"));

        let error = HotkeyError::RegistrationFailed {
            reason: "test reason".to_string(),
        };
        assert!(error.user_message().contains("test reason"));

        let error = HotkeyError::InvalidBinding;
        assert!(error.user_message().contains("not valid"));
    }

    #[test]
    fn test_hotkey_error_is_recoverable() {
        assert!(HotkeyError::PermissionDenied.is_recoverable());
        assert!(HotkeyError::InvalidBinding.is_recoverable());
        assert!(HotkeyError::ConflictDetected {
            app: "Test".to_string(),
            suggestion: "Test".to_string(),
        }
        .is_recoverable());
        assert!(!HotkeyError::RegistrationFailed {
            reason: "test".to_string()
        }
        .is_recoverable());
    }

    // -------------------------------------------------------------------------
    // Spotlight Detection Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_read_spotlight_enabled_true() {
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AppleSymbolicHotKeys</key>
    <dict>
        <key>64</key>
        <dict>
            <key>enabled</key>
            <true/>
            <key>value</key>
            <dict>
                <key>parameters</key>
                <array>
                    <integer>32</integer>
                    <integer>49</integer>
                    <integer>1048576</integer>
                </array>
                <key>type</key>
                <string>standard</string>
            </dict>
        </dict>
    </dict>
</dict>
</plist>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(plist_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = read_spotlight_enabled_status(temp_file.path());
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_read_spotlight_enabled_false() {
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AppleSymbolicHotKeys</key>
    <dict>
        <key>64</key>
        <dict>
            <key>enabled</key>
            <false/>
        </dict>
    </dict>
</dict>
</plist>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(plist_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = read_spotlight_enabled_status(temp_file.path());
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_read_spotlight_missing_key() {
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AppleSymbolicHotKeys</key>
    <dict>
        <key>65</key>
        <dict>
            <key>enabled</key>
            <true/>
        </dict>
    </dict>
</dict>
</plist>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(plist_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = read_spotlight_enabled_status(temp_file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("key 64"));
    }

    #[test]
    fn test_is_spotlight_enabled_from_path_missing_file() {
        let path = Path::new("/nonexistent/path/to/plist");
        // Should default to true (assume enabled) when file is missing
        let result = is_spotlight_enabled_from_path(path);
        assert!(result);
    }

    // -------------------------------------------------------------------------
    // Conflict Detection Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_detect_conflict_with_spotlight_enabled() {
        // Create a mock plist with Spotlight enabled
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AppleSymbolicHotKeys</key>
    <dict>
        <key>64</key>
        <dict>
            <key>enabled</key>
            <true/>
        </dict>
    </dict>
</dict>
</plist>"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(plist_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Test that conflict is detected when Spotlight is enabled
        let binding = HotkeyBinding::default(); // Cmd+Space
        let spotlight_enabled = is_spotlight_enabled_from_path(temp_file.path());
        assert!(spotlight_enabled);

        // The actual detect_hotkey_conflict uses the real system path,
        // so we test the components separately
        assert!(is_spotlight_hotkey(&binding));
    }

    #[test]
    fn test_no_conflict_with_different_hotkey() {
        // Cmd+A should not conflict with Spotlight
        let binding = HotkeyBinding::new("A", Modifiers::COMMAND);
        assert!(!is_spotlight_hotkey(&binding));
    }

    #[test]
    fn test_no_conflict_with_option_space() {
        // Option+Space should not conflict with Spotlight's Cmd+Space
        let binding = HotkeyBinding::new("Space", Modifiers::OPTION);
        assert!(!is_spotlight_hotkey(&binding));
    }

    // -------------------------------------------------------------------------
    // DoubleTapDetector Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_double_tap_detector_creation() {
        let detector = DoubleTapDetector::new(Modifier::Command, 300);
        assert_eq!(detector.target_modifier(), Modifier::Command);
        assert_eq!(detector.threshold(), Duration::from_millis(300));
    }

    #[test]
    fn test_double_tap_detector_default_threshold() {
        let detector = DoubleTapDetector::with_default_threshold(Modifier::Option);
        assert_eq!(detector.target_modifier(), Modifier::Option);
        assert_eq!(
            detector.threshold(),
            Duration::from_millis(DEFAULT_DOUBLE_TAP_THRESHOLD_MS)
        );
    }

    #[test]
    fn test_double_tap_detector_single_tap() {
        let mut detector = DoubleTapDetector::new(Modifier::Command, 300);

        // Single tap should not trigger
        assert!(!detector.on_modifier_event(Modifier::Command, true));
        assert!(!detector.on_modifier_event(Modifier::Command, false));
    }

    #[test]
    fn test_double_tap_detector_wrong_modifier() {
        let mut detector = DoubleTapDetector::new(Modifier::Command, 300);

        // Wrong modifier should not trigger
        assert!(!detector.on_modifier_event(Modifier::Option, true));
        assert!(!detector.on_modifier_event(Modifier::Option, true));
    }

    #[test]
    fn test_double_tap_detector_reset() {
        let mut detector = DoubleTapDetector::new(Modifier::Command, 500);

        // First tap
        detector.on_modifier_event(Modifier::Command, true);
        assert!(detector.has_pending_tap());

        // Reset
        detector.reset();
        assert!(!detector.has_pending_tap());
    }

    #[test]
    fn test_double_tap_detector_clone() {
        let mut detector = DoubleTapDetector::new(Modifier::Command, 300);
        detector.on_modifier_event(Modifier::Command, true);

        let cloned = detector.clone();
        // Clone should have same settings but no pending tap state
        assert_eq!(cloned.target_modifier(), Modifier::Command);
        assert_eq!(cloned.threshold(), Duration::from_millis(300));
        assert!(!cloned.has_pending_tap());
    }

    #[test]
    fn test_double_tap_detector_set_threshold() {
        let mut detector = DoubleTapDetector::new(Modifier::Command, 300);
        detector.set_threshold(500);
        assert_eq!(detector.threshold(), Duration::from_millis(500));
    }

    // -------------------------------------------------------------------------
    // HotkeyManager Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_hotkey_manager_creation() {
        let manager = HotkeyManager::new();
        assert!(!manager.is_registered());
        assert!(manager.current_binding().is_none());
        assert!(manager.double_tap_detector().is_none());
    }

    #[test]
    fn test_hotkey_manager_double_tap_detector() {
        let mut manager = HotkeyManager::new();

        let detector = DoubleTapDetector::new(Modifier::Command, 300);
        manager.set_double_tap_detector(Some(detector));

        assert!(manager.double_tap_detector().is_some());
        assert!(manager.double_tap_detector_mut().is_some());
    }

    // -------------------------------------------------------------------------
    // Modifier Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_modifier_display() {
        assert_eq!(format!("{}", Modifier::Command), "Command");
        assert_eq!(format!("{}", Modifier::Option), "Option");
        assert_eq!(format!("{}", Modifier::Control), "Control");
        assert_eq!(format!("{}", Modifier::Shift), "Shift");
    }

    #[test]
    fn test_modifiers_constants() {
        let command = Modifiers::COMMAND;
        assert!(command.command);
        assert!(!command.option);

        let option = Modifiers::OPTION;
        assert!(option.option);
        assert!(!option.command);

        let control = Modifiers::CONTROL;
        assert!(control.control);
        assert!(!control.command);

        let shift = Modifiers::SHIFT;
        assert!(shift.shift);
        assert!(!shift.command);
    }
}
