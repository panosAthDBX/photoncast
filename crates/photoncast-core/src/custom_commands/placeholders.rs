//! Placeholder expansion for custom commands.
//!
//! This module handles expanding placeholders in command strings:
//! - `{query}` - The search query text
//! - `{selection}` - Currently selected text (from frontmost app)
//! - `{clipboard}` - Contents of the clipboard
//! - `{env:VAR}` - Environment variable value

use std::borrow::Cow;
use thiserror::Error;

/// Errors that can occur during placeholder expansion.
#[derive(Error, Debug)]
pub enum PlaceholderError {
    /// Environment variable not found.
    #[error("environment variable '{name}' not found")]
    EnvNotFound { name: String },

    /// Failed to get clipboard contents.
    #[error("failed to get clipboard: {0}")]
    ClipboardError(String),

    /// Failed to get selection.
    #[error("failed to get selection: {0}")]
    SelectionError(String),

    /// Invalid placeholder syntax.
    #[error("invalid placeholder syntax: {0}")]
    InvalidSyntax(String),
}

/// Context for placeholder expansion.
#[derive(Debug, Default, Clone)]
pub struct PlaceholderContext {
    /// The search query text.
    pub query: String,
    /// The currently selected text.
    pub selection: Option<String>,
    /// The clipboard contents.
    pub clipboard: Option<String>,
}

impl PlaceholderContext {
    /// Creates a new placeholder context with just a query.
    #[must_use]
    pub fn with_query(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            selection: None,
            clipboard: None,
        }
    }

    /// Sets the selection text.
    #[must_use]
    pub fn with_selection(mut self, selection: impl Into<String>) -> Self {
        self.selection = Some(selection.into());
        self
    }

    /// Sets the clipboard contents.
    #[must_use]
    pub fn with_clipboard(mut self, clipboard: impl Into<String>) -> Self {
        self.clipboard = Some(clipboard.into());
        self
    }

    /// Lazily loads the clipboard if not already set.
    ///
    /// On macOS, uses `pbpaste` to get clipboard contents.
    pub fn ensure_clipboard(&mut self) -> Result<(), PlaceholderError> {
        if self.clipboard.is_some() {
            return Ok(());
        }

        match get_clipboard_contents() {
            Ok(contents) => {
                self.clipboard = Some(contents);
                Ok(())
            },
            Err(e) => Err(PlaceholderError::ClipboardError(e)),
        }
    }

    /// Lazily loads the selection if not already set.
    ///
    /// Note: Selection retrieval requires accessibility permissions.
    pub fn ensure_selection(&mut self) -> Result<(), PlaceholderError> {
        if self.selection.is_some() {
            return Ok(());
        }

        match get_selected_text() {
            Ok(text) => {
                self.selection = Some(text);
                Ok(())
            },
            Err(e) => Err(PlaceholderError::SelectionError(e)),
        }
    }
}

/// Expands all placeholders in a command string.
///
/// # Arguments
///
/// * `command` - The command string with placeholders.
/// * `context` - The context containing values for expansion.
///
/// # Returns
///
/// The expanded command string, or an error if expansion fails.
///
/// # Example
///
/// ```ignore
/// let ctx = PlaceholderContext::with_query("hello world");
/// let expanded = expand_placeholders("echo {query}", &ctx)?;
/// assert_eq!(expanded, "echo 'hello world'");
/// ```
pub fn expand_placeholders(
    command: &str,
    context: &PlaceholderContext,
) -> Result<String, PlaceholderError> {
    let mut result = command.to_string();
    let mut ctx = context.clone();

    // Expand {query}
    if result.contains("{query}") {
        result = result.replace("{query}", &shell_escape(&ctx.query));
    }

    // Expand {clipboard}
    if result.contains("{clipboard}") {
        ctx.ensure_clipboard()?;
        let clipboard = ctx.clipboard.as_deref().unwrap_or("");
        result = result.replace("{clipboard}", &shell_escape(clipboard));
    }

    // Expand {selection}
    if result.contains("{selection}") {
        ctx.ensure_selection()?;
        let selection = ctx.selection.as_deref().unwrap_or("");
        result = result.replace("{selection}", &shell_escape(selection));
    }

    // Expand {env:VAR} patterns
    result = expand_env_placeholders(&result)?;

    Ok(result)
}

/// Expands environment variable placeholders.
#[allow(clippy::manual_let_else, clippy::while_let_loop)]
fn expand_env_placeholders(command: &str) -> Result<String, PlaceholderError> {
    let mut result = command.to_string();

    // Find all {env:VAR} patterns and expand them
    loop {
        // Find the next {env: pattern
        let start = match result.find("{env:") {
            Some(idx) => idx,
            None => break,
        };

        // Find the closing }
        let rest = &result[start + 5..];
        let end_offset = match rest.find('}') {
            Some(idx) => idx,
            None => {
                return Err(PlaceholderError::InvalidSyntax(
                    "unclosed {env: placeholder".to_string(),
                ));
            },
        };

        let var_name = &rest[..end_offset];

        // Validate variable name (alphanumeric and underscores, starting with letter or underscore)
        if var_name.is_empty() || !is_valid_env_var_name(var_name) {
            return Err(PlaceholderError::InvalidSyntax(format!(
                "invalid environment variable name: {var_name}"
            )));
        }

        // Get the environment variable value
        let value = std::env::var(var_name).map_err(|_| PlaceholderError::EnvNotFound {
            name: var_name.to_string(),
        })?;

        // Replace the placeholder
        let full_match = format!("{{env:{var_name}}}");
        result = result.replace(&full_match, &shell_escape(&value));
    }

    Ok(result)
}

/// Checks if a string is a valid environment variable name.
fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();

    // First character must be letter or underscore
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {},
        _ => return false,
    }

    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Escapes a string for safe use in shell commands.
///
/// Wraps the string in single quotes and escapes any single quotes within.
fn shell_escape(s: &str) -> Cow<'_, str> {
    if s.is_empty() {
        return Cow::Borrowed("''");
    }

    // Check if escaping is needed
    let needs_escape = s.chars().any(|c| {
        matches!(
            c,
            ' ' | '"'
                | '\''
                | '\\'
                | '$'
                | '`'
                | '!'
                | '&'
                | '|'
                | ';'
                | '('
                | ')'
                | '<'
                | '>'
                | '\n'
                | '\t'
        )
    });

    if !needs_escape && !s.starts_with('-') {
        return Cow::Borrowed(s);
    }

    // Escape using single quotes (escape single quotes as '\'' )
    let escaped = format!("'{}'", s.replace('\'', "'\\''"));
    Cow::Owned(escaped)
}

/// Gets the contents of the system clipboard.
#[cfg(target_os = "macos")]
fn get_clipboard_contents() -> Result<String, String> {
    use std::process::Command;

    let output = Command::new("pbpaste")
        .output()
        .map_err(|e| format!("failed to run pbpaste: {e}"))?;

    if !output.status.success() {
        return Err("pbpaste failed".to_string());
    }

    String::from_utf8(output.stdout).map_err(|e| format!("clipboard contains invalid UTF-8: {e}"))
}

#[cfg(not(target_os = "macos"))]
fn get_clipboard_contents() -> Result<String, String> {
    Err("clipboard not supported on this platform".to_string())
}

/// Gets the currently selected text from the frontmost application.
///
/// Note: This requires accessibility permissions on macOS.
#[cfg(target_os = "macos")]
fn get_selected_text() -> Result<String, String> {
    use std::process::Command;

    // Use AppleScript to get selected text from frontmost app
    // This simulates Cmd+C, gets clipboard, then restores original clipboard
    let script = r#"
        use framework "AppKit"
        use scripting additions

        -- Save current clipboard
        set oldClipboard to the clipboard

        -- Copy selected text
        tell application "System Events"
            keystroke "c" using {command down}
        end tell

        -- Wait a bit for copy to complete
        delay 0.1

        -- Get new clipboard (selected text)
        set selectedText to the clipboard

        -- Restore original clipboard
        set the clipboard to oldClipboard

        return selectedText
    "#;

    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| format!("failed to run osascript: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript failed: {stderr}"));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("selection contains invalid UTF-8: {e}"))?;

    Ok(text.trim().to_string())
}

#[cfg(not(target_os = "macos"))]
fn get_selected_text() -> Result<String, String> {
    Err("selection not supported on this platform".to_string())
}

/// Checks if a command string contains any placeholders.
#[must_use]
pub fn has_placeholders(command: &str) -> bool {
    command.contains("{query}")
        || command.contains("{selection}")
        || command.contains("{clipboard}")
        || command.contains("{env:")
}

/// Lists all placeholders found in a command string.
#[must_use]
pub fn list_placeholders(command: &str) -> Vec<String> {
    let mut placeholders = Vec::new();

    if command.contains("{query}") {
        placeholders.push("{query}".to_string());
    }
    if command.contains("{selection}") {
        placeholders.push("{selection}".to_string());
    }
    if command.contains("{clipboard}") {
        placeholders.push("{clipboard}".to_string());
    }

    // Find {env:VAR} patterns
    let mut search_start = 0;
    while let Some(start) = command[search_start..].find("{env:") {
        let abs_start = search_start + start;
        if let Some(end_offset) = command[abs_start + 5..].find('}') {
            let var_name = &command[abs_start + 5..abs_start + 5 + end_offset];
            if !var_name.is_empty() && is_valid_env_var_name(var_name) {
                let placeholder = format!("{{env:{var_name}}}");
                if !placeholders.contains(&placeholder) {
                    placeholders.push(placeholder);
                }
            }
            search_start = abs_start + 5 + end_offset + 1;
        } else {
            break;
        }
    }

    placeholders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_query() {
        let ctx = PlaceholderContext::with_query("hello world");
        let result = expand_placeholders("echo {query}", &ctx).unwrap();
        assert_eq!(result, "echo 'hello world'");
    }

    #[test]
    fn test_expand_query_no_escape_needed() {
        let ctx = PlaceholderContext::with_query("simple");
        let result = expand_placeholders("echo {query}", &ctx).unwrap();
        assert_eq!(result, "echo simple");
    }

    #[test]
    fn test_expand_query_with_quotes() {
        let ctx = PlaceholderContext::with_query("it's a test");
        let result = expand_placeholders("echo {query}", &ctx).unwrap();
        assert_eq!(result, "echo 'it'\\''s a test'");
    }

    #[test]
    fn test_expand_clipboard() {
        let ctx = PlaceholderContext::with_query("").with_clipboard("clipboard content");
        let result = expand_placeholders("echo {clipboard}", &ctx).unwrap();
        assert_eq!(result, "echo 'clipboard content'");
    }

    #[test]
    fn test_expand_selection() {
        let ctx = PlaceholderContext::with_query("").with_selection("selected text");
        let result = expand_placeholders("echo {selection}", &ctx).unwrap();
        assert_eq!(result, "echo 'selected text'");
    }

    #[test]
    fn test_expand_env() {
        std::env::set_var("TEST_PLACEHOLDER_VAR", "test_value");
        let ctx = PlaceholderContext::default();
        let result = expand_placeholders("echo {env:TEST_PLACEHOLDER_VAR}", &ctx).unwrap();
        assert_eq!(result, "echo test_value");
        std::env::remove_var("TEST_PLACEHOLDER_VAR");
    }

    #[test]
    fn test_expand_env_not_found() {
        let ctx = PlaceholderContext::default();
        let result = expand_placeholders("echo {env:NONEXISTENT_VAR_12345}", &ctx);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlaceholderError::EnvNotFound { .. }
        ));
    }

    #[test]
    fn test_expand_multiple() {
        std::env::set_var("TEST_MULTI_VAR", "env_val");
        let ctx = PlaceholderContext::with_query("query_val").with_clipboard("clip_val");
        let result = expand_placeholders("{query} {clipboard} {env:TEST_MULTI_VAR}", &ctx).unwrap();
        assert!(result.contains("query_val"));
        assert!(result.contains("clip_val"));
        assert!(result.contains("env_val"));
        std::env::remove_var("TEST_MULTI_VAR");
    }

    #[test]
    fn test_shell_escape_empty() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "hello");
    }

    #[test]
    fn test_shell_escape_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_shell_escape_special_chars() {
        assert_eq!(shell_escape("$HOME"), "'$HOME'");
        assert_eq!(shell_escape("a;b"), "'a;b'");
        assert_eq!(shell_escape("a|b"), "'a|b'");
    }

    #[test]
    fn test_has_placeholders() {
        assert!(!has_placeholders("echo hello"));
        assert!(has_placeholders("echo {query}"));
        assert!(has_placeholders("echo {clipboard}"));
        assert!(has_placeholders("echo {selection}"));
        assert!(has_placeholders("echo {env:PATH}"));
    }

    #[test]
    fn test_list_placeholders() {
        let placeholders = list_placeholders("echo {query} {clipboard} {env:HOME}");
        assert_eq!(placeholders.len(), 3);
        assert!(placeholders.contains(&"{query}".to_string()));
        assert!(placeholders.contains(&"{clipboard}".to_string()));
        assert!(placeholders.contains(&"{env:HOME}".to_string()));
    }
}
