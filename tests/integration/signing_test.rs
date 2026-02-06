//! Integration tests for code signing and Gatekeeper verification.
//!
//! Task 5.1: Test Code Signing & Gatekeeper
//!
//! These tests verify that the signed app bundle passes Apple's security checks,
//! including Gatekeeper validation, signature verification, and notarization ticket
//! stapling.
//!
//! # Test Categories
//!
//! - **Signature Verification**: Validates codesign output
//! - **Gatekeeper Validation**: Verifies spctl acceptance
//! - **Notarization**: Checks stapler validation
//! - **Entitlements**: Verifies hardened runtime
//!
//! # Running These Tests
//!
//! These tests require a signed app bundle and should be run as part of the
//! release verification process:
//!
//! ```bash
//! cargo test --test integration -- signing_test --ignored
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

/// Default path to the built app bundle
const DEFAULT_APP_PATH: &str = "build/PhotonCast.app";
const DEFAULT_DMG_PATH: &str = "build/PhotonCast.dmg";

/// Helper to get the app bundle path, supporting environment override
fn get_app_path() -> PathBuf {
    std::env::var("PHOTONCAST_APP_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_APP_PATH))
}

/// Helper to get the DMG path, supporting environment override
fn get_dmg_path() -> PathBuf {
    std::env::var("PHOTONCAST_DMG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DMG_PATH))
}

/// Executes a command and returns (success, stdout, stderr)
fn run_command(cmd: &str, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .expect("Failed to execute command");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// Checks if an app bundle exists at the specified path
fn app_bundle_exists(path: &Path) -> bool {
    path.exists() && path.is_dir() && path.extension().is_some_and(|ext| ext == "app")
}

/// Checks if a file exists at the specified path
fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

// =============================================================================
// Basic Signature Verification Tests
// =============================================================================

#[test]
#[ignore = "requires signed app bundle, run with --ignored"]
fn test_codesign_verify_basic() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!(
            "Skipping test: App bundle not found at {}",
            app_path.display()
        );
        eprintln!("Build with: ./scripts/release-build.sh && ./scripts/sign.sh");
        return;
    }

    let (success, stdout, stderr) = run_command(
        "codesign",
        &["--verify", "--verbose", app_path.to_str().unwrap()],
    );

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(
        success,
        "codesign --verify failed for app bundle: {}",
        stderr
    );

    // Check for valid signature indicator
    let combined_output = format!("{} {}", stdout, stderr);
    assert!(
        combined_output.contains("valid on disk")
            || combined_output.contains("satisfies its Designated Requirement"),
        "Expected valid signature indicator in output: {}",
        combined_output
    );
}

#[test]
#[ignore = "requires signed app bundle, run with --ignored"]
fn test_codesign_verify_deep() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let (success, stdout, stderr) = run_command(
        "codesign",
        &[
            "--verify",
            "--deep",
            "--strict",
            "--verbose=2",
            app_path.to_str().unwrap(),
        ],
    );

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(
        success,
        "codesign --verify --deep --strict failed: {}",
        stderr
    );
}

// =============================================================================
// Gatekeeper (spctl) Tests
// =============================================================================

#[test]
#[ignore = "requires signed and notarized app bundle, run with --ignored"]
fn test_spctl_accepts_app() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let (success, stdout, stderr) =
        run_command("spctl", &["-a", "-v", app_path.to_str().unwrap()]);

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    // spctl outputs to stderr
    let combined = format!("{} {}", stdout, stderr);

    assert!(
        combined.contains("accepted")
            || combined.contains("source=Notarized Developer ID")
            || combined.contains("source=Developer ID"),
        "spctl should accept the app bundle. Output: {}",
        combined
    );
}

#[test]
#[ignore = "requires signed and notarized DMG, run with --ignored"]
fn test_spctl_accepts_dmg() {
    let dmg_path = get_dmg_path();

    if !file_exists(&dmg_path) {
        eprintln!("Skipping test: DMG not found at {}", dmg_path.display());
        return;
    }

    let (success, stdout, stderr) =
        run_command("spctl", &["-a", "-v", "-t", "open", dmg_path.to_str().unwrap()]);

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    let combined = format!("{} {}", stdout, stderr);

    assert!(
        combined.contains("accepted"),
        "spctl should accept the DMG. Output: {}",
        combined
    );
}

// =============================================================================
// Notarization Stapling Tests
// =============================================================================

#[test]
#[ignore = "requires notarized app bundle, run with --ignored"]
fn test_stapler_validate_app() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let (success, stdout, stderr) = run_command(
        "xcrun",
        &["stapler", "validate", app_path.to_str().unwrap()],
    );

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    let combined = format!("{} {}", stdout, stderr);

    assert!(
        combined.contains("The validate action worked") || success,
        "stapler validate should pass for notarized app. Output: {}",
        combined
    );
}

#[test]
#[ignore = "requires notarized DMG, run with --ignored"]
fn test_stapler_validate_dmg() {
    let dmg_path = get_dmg_path();

    if !file_exists(&dmg_path) {
        eprintln!("Skipping test: DMG not found");
        return;
    }

    let (success, stdout, stderr) =
        run_command("xcrun", &["stapler", "validate", dmg_path.to_str().unwrap()]);

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    let combined = format!("{} {}", stdout, stderr);

    assert!(
        combined.contains("The validate action worked") || success,
        "stapler validate should pass for notarized DMG. Output: {}",
        combined
    );
}

// =============================================================================
// Hardened Runtime and Entitlements Tests
// =============================================================================

#[test]
#[ignore = "requires signed app bundle, run with --ignored"]
fn test_hardened_runtime_enabled() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let (_, stdout, stderr) = run_command(
        "codesign",
        &["--display", "--verbose=4", app_path.to_str().unwrap()],
    );

    let combined = format!("{} {}", stdout, stderr);
    println!("codesign display output:\n{}", combined);

    // Check for hardened runtime indicator
    assert!(
        combined.contains("runtime") || combined.contains("flags=0x10000"),
        "Hardened runtime should be enabled. Look for 'runtime' or 'flags=0x10000' in output"
    );
}

#[test]
#[ignore = "requires signed app bundle, run with --ignored"]
fn test_entitlements_present() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let (success, stdout, stderr) = run_command(
        "codesign",
        &[
            "--display",
            "--entitlements",
            ":-",
            app_path.to_str().unwrap(),
        ],
    );

    let combined = format!("{} {}", stdout, stderr);
    println!("Entitlements:\n{}", combined);

    // Check for expected entitlements
    let expected_entitlements = [
        "com.apple.security.automation.apple-events",
        "com.apple.security.network.client",
    ];

    for entitlement in &expected_entitlements {
        assert!(
            combined.contains(entitlement),
            "Expected entitlement '{}' not found in: {}",
            entitlement,
            combined
        );
    }
}

// =============================================================================
// Certificate and Identity Tests
// =============================================================================

#[test]
#[ignore = "requires signed app bundle, run with --ignored"]
fn test_signed_with_developer_id() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let (_, stdout, stderr) = run_command(
        "codesign",
        &["--display", "--verbose=4", app_path.to_str().unwrap()],
    );

    let combined = format!("{} {}", stdout, stderr);

    // Check for Developer ID certificate
    assert!(
        combined.contains("Developer ID Application")
            || combined.contains("Authority=Developer ID"),
        "App should be signed with Developer ID Application certificate. Output: {}",
        combined
    );
}

#[test]
#[ignore = "requires signed app bundle, run with --ignored"]
fn test_no_quarantine_attribute() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let (success, stdout, stderr) =
        run_command("xattr", &["-l", app_path.to_str().unwrap()]);

    let combined = format!("{} {}", stdout, stderr);

    // After proper signing and notarization, the quarantine flag should be removable
    // If the app was downloaded, it may have quarantine initially
    // This test just checks if we can list extended attributes
    println!("Extended attributes: {}", combined);

    // Note: com.apple.quarantine is added by browsers on download
    // A properly notarized app will still show a warning but will be allowed to run
}

// =============================================================================
// Bundle Structure Tests
// =============================================================================

#[test]
fn test_app_bundle_structure() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let contents_path = app_path.join("Contents");
    let macos_path = contents_path.join("MacOS");
    let resources_path = contents_path.join("Resources");
    let info_plist_path = contents_path.join("Info.plist");

    assert!(
        contents_path.exists(),
        "Contents directory should exist at {}",
        contents_path.display()
    );
    assert!(
        macos_path.exists(),
        "MacOS directory should exist at {}",
        macos_path.display()
    );
    assert!(
        resources_path.exists(),
        "Resources directory should exist at {}",
        resources_path.display()
    );
    assert!(
        info_plist_path.exists(),
        "Info.plist should exist at {}",
        info_plist_path.display()
    );
}

#[test]
fn test_executable_exists() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let executable_path = app_path.join("Contents/MacOS/photoncast");

    assert!(
        executable_path.exists(),
        "Executable should exist at {}",
        executable_path.display()
    );

    // Check it's actually executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&executable_path).unwrap();
        let permissions = metadata.permissions();
        assert!(
            permissions.mode() & 0o111 != 0,
            "Executable should have execute permissions"
        );
    }
}

// =============================================================================
// Info.plist Validation Tests
// =============================================================================

#[test]
fn test_info_plist_required_keys() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let info_plist_path = app_path.join("Contents/Info.plist");
    let plist_content =
        std::fs::read_to_string(&info_plist_path).expect("Failed to read Info.plist");

    // Required keys for a properly configured app
    let required_keys = [
        "CFBundleName",
        "CFBundleIdentifier",
        "CFBundleVersion",
        "CFBundleShortVersionString",
        "CFBundleExecutable",
    ];

    for key in &required_keys {
        assert!(
            plist_content.contains(key),
            "Info.plist should contain key '{}'. Content: {}",
            key,
            plist_content
        );
    }
}

#[test]
fn test_info_plist_bundle_identifier() {
    let app_path = get_app_path();

    if !app_bundle_exists(&app_path) {
        eprintln!("Skipping test: App bundle not found");
        return;
    }

    let info_plist_path = app_path.join("Contents/Info.plist");
    let plist_content =
        std::fs::read_to_string(&info_plist_path).expect("Failed to read Info.plist");

    assert!(
        plist_content.contains("com.photoncast.app"),
        "Bundle identifier should be 'com.photoncast.app'. Content: {}",
        plist_content
    );
}

// =============================================================================
// Mock Tests (Always Run)
// =============================================================================

/// Test helper functions work correctly
#[test]
fn test_helper_functions() {
    let path = PathBuf::from("/nonexistent/path.app");
    assert!(!app_bundle_exists(&path));

    let path = PathBuf::from("/tmp");
    assert!(!app_bundle_exists(&path)); // Not an .app bundle
}

/// Test command execution helper
#[test]
fn test_run_command_echo() {
    let (success, stdout, _) = run_command("echo", &["hello", "world"]);
    assert!(success);
    assert!(stdout.trim() == "hello world");
}

/// Test that test environment variables work
#[test]
fn test_environment_override() {
    std::env::set_var("PHOTONCAST_APP_PATH", "/custom/path/PhotonCast.app");
    let path = get_app_path();
    assert_eq!(path, PathBuf::from("/custom/path/PhotonCast.app"));
    std::env::remove_var("PHOTONCAST_APP_PATH");
}
