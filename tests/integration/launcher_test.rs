//! Integration tests for launcher functionality.
//!
//! These tests cover security measures and helper functions.

/// Escapes a path string for safe use in AppleScript.
/// This is a copy of the function from launcher.rs for testing.
fn escape_path_for_applescript(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

// ============================================================================
// AppleScript Path Escaping Tests (Security)
// ============================================================================

#[test]
fn test_applescript_path_escape_normal_path() {
    let path = "/Applications/Safari.app";
    let escaped = escape_path_for_applescript(path);
    assert_eq!(escaped, "/Applications/Safari.app");
}

#[test]
fn test_applescript_path_escape_with_quotes() {
    let path = r#"/Users/test/My "Documents"/file.txt"#;
    let escaped = escape_path_for_applescript(path);
    assert_eq!(escaped, r#"/Users/test/My \"Documents\"/file.txt"#);
}

#[test]
fn test_applescript_path_escape_with_backslash() {
    let path = r"/Users/test/path\with\backslashes";
    let escaped = escape_path_for_applescript(path);
    assert_eq!(escaped, r"/Users/test/path\\with\\backslashes");
}

#[test]
fn test_applescript_path_escape_injection_attempt() {
    // Attempt to inject AppleScript commands
    let path = r#"/tmp/"); do shell script "malicious"; --"#;
    let escaped = escape_path_for_applescript(path);
    // The quotes should be escaped, preventing injection
    assert_eq!(escaped, r#"/tmp/\"); do shell script \"malicious\"; --"#);
    // Verify the escaped string can be safely used in AppleScript
    let script = format!(r#"set the clipboard to (POSIX file "{}")"#, escaped);
    assert!(script.contains(r#"\"malicious\""#));
}

#[test]
fn test_applescript_path_escape_unicode() {
    let path = "/Users/用户/Documents/文件.txt";
    let escaped = escape_path_for_applescript(path);
    assert_eq!(escaped, "/Users/用户/Documents/文件.txt");
}

#[test]
fn test_applescript_path_escape_empty_string() {
    let path = "";
    let escaped = escape_path_for_applescript(path);
    assert_eq!(escaped, "");
}

#[test]
fn test_applescript_path_escape_spaces() {
    let path = "/Users/test/My Documents/file.txt";
    let escaped = escape_path_for_applescript(path);
    assert_eq!(escaped, "/Users/test/My Documents/file.txt");
}
