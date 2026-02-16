//! Code signing verification for extension dylibs.
//!
//! Verifies that extension dylibs have valid code signatures before loading.
//! In dev mode, verification is skipped with a warning.
//!
//! Optionally verifies the signing identity (TeamIdentifier) against an
//! allowlist to ensure extensions are signed by trusted developers.

use std::path::Path;
use std::process::Command;

/// Configuration for extension code signing verification.
#[derive(Debug, Clone, Default)]
pub struct ExtensionSigningConfig {
    /// Allowed Apple Team Identifiers. If non-empty and `require_identity` is true,
    /// only extensions signed by one of these teams will be accepted.
    pub allowed_team_ids: Vec<String>,
    /// Whether to require identity (TeamIdentifier) verification in addition to
    /// basic signature validation. When false, only `codesign --verify` is checked.
    pub require_identity: bool,
}

/// Verifies the code signature of an extension dylib.
///
/// On macOS, uses `codesign --verify` to check the signature.
/// Returns `Ok(())` if the signature is valid, or an error describing the failure.
///
/// This function does NOT verify signing identity. Use
/// [`verify_code_signature_with_config`] to also check the TeamIdentifier.
pub fn verify_code_signature(path: &Path) -> Result<(), CodeSignatureError> {
    verify_basic_signature(path)
}

/// Verifies the code signature and optionally the signing identity.
///
/// When `config.require_identity` is true, this also extracts the
/// TeamIdentifier via `codesign -dvvv` and checks it against
/// `config.allowed_team_ids`.
///
/// When `config.require_identity` is false, this behaves identically to
/// [`verify_code_signature`].
pub fn verify_code_signature_with_config(
    path: &Path,
    config: &ExtensionSigningConfig,
) -> Result<(), CodeSignatureError> {
    // Always perform basic signature verification first.
    verify_basic_signature(path)?;

    // Optionally verify the signing identity.
    if config.require_identity {
        let refs: Vec<&str> = config.allowed_team_ids.iter().map(String::as_str).collect();
        verify_code_signature_identity(path, &refs)?;
    }

    Ok(())
}

/// Verifies that the code signature on `path` was produced by a team whose
/// TeamIdentifier is in `allowed_team_ids`.
///
/// Uses `codesign -dvvv` to extract signing information and parses the
/// `TeamIdentifier=` line from stderr output.
///
/// # Errors
///
/// Returns an error if:
/// - `codesign` cannot be executed
/// - No TeamIdentifier is found in the output
/// - The TeamIdentifier is `"not set"`
/// - The TeamIdentifier is not in `allowed_team_ids`
pub fn verify_code_signature_identity(
    path: &Path,
    allowed_team_ids: &[&str],
) -> Result<(), CodeSignatureError> {
    if allowed_team_ids.is_empty() {
        return Err(CodeSignatureError::IdentityVerificationFailed(
            "no allowed team IDs configured".to_string(),
        ));
    }

    let output = Command::new("codesign")
        .args(["-dvvv"])
        .arg(path)
        .output()
        .map_err(|e| {
            CodeSignatureError::VerificationFailed(format!("Failed to run codesign -dvvv: {e}"))
        })?;

    // codesign -dvvv writes signing info to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);

    let team_id = stderr
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("TeamIdentifier=").map(str::trim)
        })
        .ok_or_else(|| {
            CodeSignatureError::IdentityVerificationFailed(format!(
                "no TeamIdentifier found in codesign output for {}",
                path.display()
            ))
        })?;

    if team_id == "not set" {
        return Err(CodeSignatureError::IdentityVerificationFailed(format!(
            "TeamIdentifier is not set for {}",
            path.display()
        )));
    }

    if !allowed_team_ids.contains(&team_id) {
        return Err(CodeSignatureError::UntrustedTeamIdentifier {
            team_id: team_id.to_string(),
            path: path.display().to_string(),
        });
    }

    Ok(())
}

/// Performs basic `codesign --verify` signature check.
fn verify_basic_signature(path: &Path) -> Result<(), CodeSignatureError> {
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
    #[error("Code signature identity verification failed: {0}")]
    IdentityVerificationFailed(String),
    #[error("Untrusted TeamIdentifier '{team_id}' for {path}")]
    UntrustedTeamIdentifier { team_id: String, path: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_verify_code_signature_nonexistent_path() {
        let path = std::path::Path::new("/tmp/photoncast_test_nonexistent_dylib_abc123.dylib");
        let result = verify_code_signature(path);
        assert!(result.is_err(), "Expected error for non-existent path");
    }

    #[test]
    fn test_verify_code_signature_unsigned_file() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let file_path = dir.path().join("unsigned.dylib");
        let mut f = std::fs::File::create(&file_path).expect("failed to create temp file");
        f.write_all(b"not a real dylib").expect("failed to write");
        drop(f);

        let result = verify_code_signature(&file_path);
        assert!(result.is_err(), "Expected error for unsigned/invalid file");
    }

    #[test]
    fn test_extension_signing_config_defaults() {
        let config = ExtensionSigningConfig::default();
        assert!(config.allowed_team_ids.is_empty());
        assert!(!config.require_identity);
    }

    #[test]
    fn test_extension_signing_config_creation() {
        let config = ExtensionSigningConfig {
            allowed_team_ids: vec!["ABC123".to_string(), "DEF456".to_string()],
            require_identity: true,
        };
        assert_eq!(config.allowed_team_ids.len(), 2);
        assert!(config.require_identity);
    }

    #[test]
    fn test_verify_with_config_require_identity_false() {
        // With require_identity=false, should behave like basic verify (still fails for bad file)
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let file_path = dir.path().join("unsigned2.dylib");
        std::fs::write(&file_path, b"fake content").expect("failed to write");

        let config = ExtensionSigningConfig {
            allowed_team_ids: vec![],
            require_identity: false,
        };
        let result = verify_code_signature_with_config(&file_path, &config);
        assert!(
            result.is_err(),
            "Expected error for unsigned file even with require_identity=false"
        );
    }

    #[test]
    fn test_verify_identity_empty_allowed_team_ids() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let file_path = dir.path().join("some.dylib");
        std::fs::write(&file_path, b"fake").expect("failed to write");

        let result = verify_code_signature_identity(&file_path, &[]);
        assert!(
            result.is_err(),
            "Expected error with empty allowed_team_ids"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("no allowed team IDs"),
            "Error should mention no allowed team IDs, got: {err_msg}"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_verify_with_config_rejects_when_identity_required_but_trusted_refs_empty() {
        let signed_path = ["/bin/ls", "/usr/bin/ls", "/bin/sh"]
            .into_iter()
            .map(std::path::Path::new)
            .find(|candidate| candidate.exists())
            .expect("expected at least one signed system binary to exist");

        verify_code_signature(signed_path)
            .expect("precondition failed: expected system binary to pass code signature check");

        let config = ExtensionSigningConfig {
            allowed_team_ids: Vec::new(),
            require_identity: true,
        };

        let error = verify_code_signature_with_config(signed_path, &config)
            .expect_err("expected identity verification to fail with empty trusted refs");

        match error {
            CodeSignatureError::IdentityVerificationFailed(message) => {
                assert!(message.contains("no allowed team IDs"));
            },
            other => panic!("expected IdentityVerificationFailed, got: {other}"),
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_verify_with_config_identity_check_bypassed_when_not_required() {
        let signed_path = ["/bin/ls", "/usr/bin/ls", "/bin/sh"]
            .into_iter()
            .map(std::path::Path::new)
            .find(|candidate| candidate.exists())
            .expect("expected at least one signed system binary to exist");

        verify_code_signature(signed_path)
            .expect("precondition failed: expected system binary to pass code signature check");

        let config = ExtensionSigningConfig {
            allowed_team_ids: Vec::new(),
            require_identity: false,
        };

        verify_code_signature_with_config(signed_path, &config)
            .expect("expected identity verification to be skipped when require_identity is false");
    }
}
