use crate::error::{AppError, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Deserialize)]
pub struct PrivilegedResponse {
    pub ok: bool,
    #[serde(rename = "requestID")]
    pub request_id: String,
    pub code: String,
    pub message: String,
    pub operation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegedUninstallMode {
    TrashFirst,
    DeleteConfirmed,
}

impl PrivilegedUninstallMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::TrashFirst => "trashFirst",
            Self::DeleteConfirmed => "deleteConfirmed",
        }
    }
}

pub fn uninstall_with_privileges(
    path: &Path,
    mode: PrivilegedUninstallMode,
) -> Result<PrivilegedResponse> {
    let client = privileged_client_path()?;
    let first = run_client_uninstall(&client, path, mode)?;

    if first.ok {
        return Ok(first);
    }

    if first.code == "xpc-error" || first.code == "helper-unavailable" {
        bless_helper(&client)?;
        let retry = run_client_uninstall(&client, path, mode)?;
        return response_to_result(retry);
    }

    response_to_result(first)
}

fn run_client_uninstall(
    client: &Path,
    path: &Path,
    mode: PrivilegedUninstallMode,
) -> Result<PrivilegedResponse> {
    let output = Command::new(client)
        .arg("uninstall")
        .arg(path)
        .arg(mode.as_str())
        .output()
        .map_err(|error| {
            AppError::PrivilegedUnavailable(format!(
                "failed to run {}: {}",
                client.display(),
                error
            ))
        })?;

    parse_response(&output)
}

fn bless_helper(client: &Path) -> Result<()> {
    let output = Command::new(client)
        .arg("bless")
        .output()
        .map_err(|error| {
            AppError::PrivilegedUnavailable(format!("failed to run bless: {error}"))
        })?;

    let response = parse_response(&output)?;
    if response.ok {
        Ok(())
    } else {
        Err(AppError::PrivilegedFailed(format!(
            "{}: {}",
            response.code, response.message
        )))
    }
}

fn parse_response(output: &Output) -> Result<PrivilegedResponse> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    serde_json::from_str(stdout.trim()).map_err(|error| {
        AppError::PrivilegedFailed(format!(
            "invalid helper response: {}; status={}; stdout={}; stderr={}",
            error,
            output.status,
            stdout.trim(),
            stderr.trim()
        ))
    })
}

fn response_to_result(response: PrivilegedResponse) -> Result<PrivilegedResponse> {
    if response.ok {
        Ok(response)
    } else {
        Err(AppError::PrivilegedFailed(format!(
            "{}: {}",
            response.code, response.message
        )))
    }
}

fn privileged_client_path() -> Result<PathBuf> {
    let exe = std::env::current_exe().map_err(|error| {
        AppError::PrivilegedUnavailable(format!("cannot locate current executable: {error}"))
    })?;
    let macos_dir = exe.parent().ok_or_else(|| {
        AppError::PrivilegedUnavailable("current executable has no parent directory".to_string())
    })?;
    let client = macos_dir.join("photoncast-privileged-client");
    if client.is_file() {
        Ok(client)
    } else {
        Err(AppError::PrivilegedUnavailable(format!(
            "missing privileged client at {}",
            client.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privileged_uninstall_mode_strings() {
        assert_eq!(PrivilegedUninstallMode::TrashFirst.as_str(), "trashFirst");
        assert_eq!(
            PrivilegedUninstallMode::DeleteConfirmed.as_str(),
            "deleteConfirmed"
        );
    }

    #[test]
    fn test_parse_privileged_response() {
        let response: PrivilegedResponse = serde_json::from_str(
            r#"{"ok":true,"requestID":"abc","code":"trashed","message":"moved","operation":"trash"}"#,
        )
        .unwrap();

        assert!(response.ok);
        assert_eq!(response.request_id, "abc");
        assert_eq!(response.operation.as_deref(), Some("trash"));
    }
}
