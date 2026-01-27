//! Code signing verification for extension dylibs.
//!
//! Verifies that extension dylibs have valid code signatures before loading.
//! In dev mode, verification is skipped with a warning.

use std::path::Path;
use std::process::Command;

/// Verifies the code signature of an extension dylib.
///
/// On macOS, uses `codesign --verify` to check the signature.
/// Returns `Ok(())` if the signature is valid, or an error describing the failure.
pub fn verify_code_signature(path: &Path) -> Result<(), CodeSignatureError> {
    let output = Command::new("codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(path)
        .output()
        .map_err(|e| {
            CodeSignatureError::VerificationFailed(format!("Failed to run codesign: {e}"))
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(CodeSignatureError::InvalidSignature(format!(
            "Code signature verification failed for {}: {}",
            path.display(),
            stderr.trim()
        )))
    }
}

/// Errors from code signing verification.
#[derive(Debug, thiserror::Error)]
pub enum CodeSignatureError {
    #[error("Code signature verification failed: {0}")]
    VerificationFailed(String),
    #[error("Invalid code signature: {0}")]
    InvalidSignature(String),
}
