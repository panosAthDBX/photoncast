//! Shared test utilities for PhotonCast integration tests.

use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a temporary directory for test data.
pub fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp directory")
}

/// Creates a mock application bundle in the given directory.
///
/// # Arguments
///
/// * `dir` - The directory to create the bundle in.
/// * `name` - The name of the application (without .app extension).
/// * `bundle_id` - The bundle identifier.
///
/// # Returns
///
/// The path to the created .app bundle.
pub fn create_mock_app(dir: &std::path::Path, name: &str, bundle_id: &str) -> PathBuf {
    let app_path = dir.join(format!("{name}.app"));
    let contents_path = app_path.join("Contents");
    let resources_path = contents_path.join("Resources");

    std::fs::create_dir_all(&resources_path).expect("failed to create app directories");

    // Create Info.plist
    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
</dict>
</plist>"#
    );

    std::fs::write(contents_path.join("Info.plist"), info_plist)
        .expect("failed to write Info.plist");

    app_path
}

/// Asserts that a condition is true, with a custom panic message.
#[macro_export]
macro_rules! assert_with_context {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            panic!("Assertion failed: {}", format!($($arg)*));
        }
    };
}

/// Test configuration for integration tests.
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Whether to use an in-memory database.
    pub use_memory_db: bool,
    /// Timeout for async operations.
    pub timeout_ms: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            use_memory_db: true,
            timeout_ms: 5000,
        }
    }
}
