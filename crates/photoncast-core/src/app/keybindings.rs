//! Keyboard shortcut management.
//!
//! This module handles custom keyboard shortcuts and keybindings configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info};

/// Default keybindings file name.
pub const KEYBINDINGS_FILE_NAME: &str = "keybindings.toml";

/// Errors that can occur during keybindings operations.
#[derive(Debug, Error)]
pub enum KeybindingsError {
    /// Failed to read the keybindings file.
    #[error("Failed to read keybindings file: {0}")]
    ReadError(#[from] io::Error),

    /// Failed to parse the keybindings file.
    #[error("Failed to parse keybindings file: {0}")]
    ParseError(#[from] toml::de::Error),

    /// Failed to serialize the keybindings.
    #[error("Failed to serialize keybindings: {0}")]
    SerializeError(#[from] toml::ser::Error),

    /// Keybindings directory not found.
    #[error("Could not determine keybindings directory")]
    NoKeybindingsDirectory,

    /// Shortcut conflict detected.
    #[error("Shortcut conflict: {0}")]
    ShortcutConflict(String),
}

/// Result type for keybindings operations.
pub type KeybindingsResult<T> = Result<T, KeybindingsError>;

/// Keyboard shortcut definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Shortcut {
    /// Key to press (e.g., "Space", "V", "1").
    pub key: String,
    /// Modifiers (e.g., `["Command", "Shift"]`).
    #[serde(default)]
    pub modifiers: Vec<String>,
}

impl Shortcut {
    /// Creates a new shortcut.
    pub fn new(key: impl Into<String>, modifiers: Vec<String>) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    /// Parses a shortcut from a string like "Cmd+Shift+V".
    pub fn from_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        let key = (*parts.last()?).to_string();
        let modifiers = parts[..parts.len() - 1]
            .iter()
            .map(|m| normalize_modifier(m))
            .collect();

        Some(Self { key, modifiers })
    }

    /// Returns a human-readable string representation.
    pub fn as_string(&self) -> String {
        if self.modifiers.is_empty() {
            self.key.clone()
        } else {
            format!("{}+{}", self.modifiers.join("+"), self.key)
        }
    }

    /// Checks if this shortcut uses the Hyper key (Cmd+Ctrl+Opt+Shift).
    pub fn uses_hyper_key(&self) -> bool {
        self.modifiers.contains(&"Command".to_string())
            && self.modifiers.contains(&"Control".to_string())
            && self.modifiers.contains(&"Option".to_string())
            && self.modifiers.contains(&"Shift".to_string())
    }
}

impl std::fmt::Display for Shortcut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

/// Normalizes modifier names (Cmd → Command, Ctrl → Control, etc.).
fn normalize_modifier(modifier: &str) -> String {
    match modifier.to_lowercase().as_str() {
        "cmd" | "command" | "⌘" => "Command".to_string(),
        "ctrl" | "control" | "⌃" => "Control".to_string(),
        "opt" | "option" | "alt" | "⌥" => "Option".to_string(),
        "shift" | "⇧" => "Shift".to_string(),
        _ => modifier.to_string(),
    }
}

/// Keybindings configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Keybindings {
    /// Global launcher hotkey.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_hotkey: Option<Shortcut>,

    /// Clipboard history hotkey.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipboard_history: Option<Shortcut>,

    /// Custom command shortcuts.
    #[serde(default)]
    pub commands: HashMap<String, Shortcut>,

    /// Window management shortcuts.
    #[serde(default)]
    pub window_management: HashMap<String, Shortcut>,
}

impl Keybindings {
    /// Creates a new empty keybindings configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads keybindings from the default path.
    pub fn load() -> KeybindingsResult<Self> {
        let path = default_keybindings_path().ok_or(KeybindingsError::NoKeybindingsDirectory)?;
        Self::load_from(&path)
    }

    /// Loads keybindings from a specific file.
    pub fn load_from(path: &Path) -> KeybindingsResult<Self> {
        if path.exists() {
            debug!(path = %path.display(), "Loading keybindings file");
            let contents = fs::read_to_string(path)?;
            let keybindings: Self = toml::from_str(&contents)?;
            info!(path = %path.display(), "Keybindings file loaded");
            Ok(keybindings)
        } else {
            debug!(path = %path.display(), "Keybindings file not found, using defaults");
            Ok(Self::default())
        }
    }

    /// Saves keybindings to the default path.
    pub fn save(&self) -> KeybindingsResult<()> {
        let path = default_keybindings_path().ok_or(KeybindingsError::NoKeybindingsDirectory)?;
        self.save_to(&path)
    }

    /// Saves keybindings to a specific file.
    pub fn save_to(&self, path: &Path) -> KeybindingsResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        info!(path = %path.display(), "Keybindings file saved");
        Ok(())
    }

    /// Sets a shortcut for a command.
    pub fn set_command_shortcut(&mut self, command_id: impl Into<String>, shortcut: Shortcut) {
        self.commands.insert(command_id.into(), shortcut);
    }

    /// Gets a shortcut for a command.
    pub fn get_command_shortcut(&self, command_id: &str) -> Option<&Shortcut> {
        self.commands.get(command_id)
    }

    /// Removes a shortcut for a command.
    pub fn remove_command_shortcut(&mut self, command_id: &str) {
        self.commands.remove(command_id);
    }

    /// Sets a window management shortcut.
    pub fn set_window_shortcut(&mut self, layout_id: impl Into<String>, shortcut: Shortcut) {
        self.window_management.insert(layout_id.into(), shortcut);
    }

    /// Gets a window management shortcut.
    pub fn get_window_shortcut(&self, layout_id: &str) -> Option<&Shortcut> {
        self.window_management.get(layout_id)
    }

    /// Checks for conflicts in shortcuts.
    ///
    /// Returns the first conflicting command IDs if found.
    pub fn check_conflicts(&self) -> Option<(String, String)> {
        let mut seen = HashMap::new();

        // Check global shortcuts
        if let Some(ref shortcut) = self.global_hotkey {
            seen.insert(shortcut, "global_hotkey".to_string());
        }
        if let Some(ref shortcut) = self.clipboard_history {
            if let Some(existing) = seen.get(shortcut) {
                return Some((existing.clone(), "clipboard_history".to_string()));
            }
            seen.insert(shortcut, "clipboard_history".to_string());
        }

        // Check command shortcuts
        for (command_id, shortcut) in &self.commands {
            if let Some(existing) = seen.get(shortcut) {
                return Some((existing.clone(), command_id.clone()));
            }
            seen.insert(shortcut, command_id.clone());
        }

        // Check window management shortcuts
        for (layout_id, shortcut) in &self.window_management {
            if let Some(existing) = seen.get(shortcut) {
                return Some((existing.clone(), layout_id.clone()));
            }
            seen.insert(shortcut, layout_id.clone());
        }

        None
    }

    /// Resets all keybindings to defaults.
    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
    }
}

/// Returns the default keybindings file path.
pub fn default_keybindings_path() -> Option<PathBuf> {
    crate::app::config_file::default_config_dir().map(|p| p.join(KEYBINDINGS_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_shortcut_from_string() {
        let shortcut = Shortcut::from_string("Cmd+Shift+V").unwrap();
        assert_eq!(shortcut.key, "V");
        assert_eq!(shortcut.modifiers, vec!["Command", "Shift"]);

        let shortcut = Shortcut::from_string("Space").unwrap();
        assert_eq!(shortcut.key, "Space");
        assert!(shortcut.modifiers.is_empty());
    }

    #[test]
    fn test_shortcut_as_string() {
        let shortcut = Shortcut::new("V", vec!["Command".to_string(), "Shift".to_string()]);
        assert_eq!(shortcut.as_string(), "Command+Shift+V");
    }

    #[test]
    fn test_shortcut_hyper_key() {
        let shortcut = Shortcut::new(
            "A",
            vec![
                "Command".to_string(),
                "Control".to_string(),
                "Option".to_string(),
                "Shift".to_string(),
            ],
        );
        assert!(shortcut.uses_hyper_key());

        let shortcut = Shortcut::new("A", vec!["Command".to_string()]);
        assert!(!shortcut.uses_hyper_key());
    }

    #[test]
    fn test_normalize_modifier() {
        assert_eq!(normalize_modifier("cmd"), "Command");
        assert_eq!(normalize_modifier("Cmd"), "Command");
        assert_eq!(normalize_modifier("⌘"), "Command");
        assert_eq!(normalize_modifier("ctrl"), "Control");
        assert_eq!(normalize_modifier("opt"), "Option");
        assert_eq!(normalize_modifier("alt"), "Option");
    }

    #[test]
    fn test_keybindings_default() {
        let keybindings = Keybindings::default();
        assert!(keybindings.global_hotkey.is_none());
        assert!(keybindings.clipboard_history.is_none());
        assert!(keybindings.commands.is_empty());
    }

    #[test]
    fn test_keybindings_set_get() {
        let mut keybindings = Keybindings::new();
        let shortcut = Shortcut::new("S", vec!["Command".to_string()]);

        keybindings.set_command_shortcut("save", shortcut.clone());
        assert_eq!(keybindings.get_command_shortcut("save"), Some(&shortcut));

        keybindings.remove_command_shortcut("save");
        assert_eq!(keybindings.get_command_shortcut("save"), None);
    }

    #[test]
    fn test_keybindings_conflict_detection() {
        let mut keybindings = Keybindings::new();
        let shortcut = Shortcut::new("S", vec!["Command".to_string()]);

        keybindings.set_command_shortcut("save", shortcut.clone());
        keybindings.set_command_shortcut("search", shortcut);

        let conflict = keybindings.check_conflicts();
        assert!(conflict.is_some());
        let (cmd1, cmd2) = conflict.unwrap();
        assert!(cmd1 == "save" || cmd2 == "save");
        assert!(cmd1 == "search" || cmd2 == "search");
    }

    #[test]
    fn test_keybindings_serialization() {
        let mut keybindings = Keybindings::new();
        keybindings.global_hotkey = Some(Shortcut::new("Space", vec!["Command".to_string()]));
        keybindings.set_command_shortcut("save", Shortcut::new("S", vec!["Command".to_string()]));

        let toml = toml::to_string(&keybindings).unwrap();
        let parsed: Keybindings = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.global_hotkey, keybindings.global_hotkey);
        assert_eq!(
            parsed.get_command_shortcut("save"),
            keybindings.get_command_shortcut("save")
        );
    }

    #[test]
    fn test_keybindings_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("keybindings.toml");

        let mut keybindings = Keybindings::new();
        keybindings.global_hotkey = Some(Shortcut::new("Space", vec!["Command".to_string()]));
        keybindings.save_to(&path).unwrap();

        let loaded = Keybindings::load_from(&path).unwrap();
        assert_eq!(loaded.global_hotkey, keybindings.global_hotkey);
    }

    #[test]
    fn test_keybindings_load_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("missing.toml");

        let keybindings = Keybindings::load_from(&path).unwrap();
        assert!(keybindings.global_hotkey.is_none());
    }

    #[test]
    fn test_keybindings_reset() {
        let mut keybindings = Keybindings::new();
        keybindings.set_command_shortcut("save", Shortcut::new("S", vec!["Command".to_string()]));

        keybindings.reset_to_defaults();
        assert!(keybindings.commands.is_empty());
    }
}
