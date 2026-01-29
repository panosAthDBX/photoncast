use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use abi_stable::external_types::RawValueBox;
use abi_stable::std_types::{ROption, RString, RVec};
use photoncast_extension_api::{
    ApplicationInfo, CommandArguments as ApiCommandArguments, ExtensionApiResult,
    ExtensionHostProtocol, Toast, ToastStyle,
};
use photoncast_extension_ipc::messages::{
    ClipboardCopyRequest, CommandArguments, CommandResponse, FrontmostApplicationInfo,
    FrontmostApplicationResponse, HudRequest, LaunchCommandRequest, OpenPathRequest,
    OpenUrlRequest, RenderViewRequest, RenderViewResponse, SetErrorRequest, SetLoadingRequest,
    TextResponse, ToastPayload, ToastRequest, ToastStylePayload, UpdateItemsRequest,
    UpdateViewRequest,
};
use photoncast_extension_ipc::methods::*;
use photoncast_extension_ipc::{IpcError, RpcConnection, RpcHandler};
use thiserror::Error;

use crate::extensions::host::ExtensionHostImpl;
use crate::extensions::manifest::ExtensionManifest;

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("sandbox runner not found at {path}")]
    RunnerNotFound { path: PathBuf },
    #[error("sandbox manifest directory missing for {extension_id}")]
    MissingManifestDir { extension_id: String },
    #[error("failed to spawn sandbox runner: {0}")]
    SpawnFailed(#[from] std::io::Error),
    #[error("sandbox stdio unavailable")]
    MissingStdio,
}

pub struct SandboxedExtension {
    pub process: Child,
    pub connection: RpcConnection,
}

pub struct HostRpcHandler {
    host: ExtensionHostImpl,
}

impl HostRpcHandler {
    #[must_use]
    pub fn new(host: ExtensionHostImpl) -> Self {
        Self { host }
    }

    fn handle_method(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, IpcError> {
        match method {
            HOST_RENDER_VIEW => {
                let request: RenderViewRequest = serde_json::from_value(params)?;
                let handle = self.host.render_view_handle(request.view);
                let response = RenderViewResponse {
                    handle_id: handle.id().value(),
                };
                Ok(serde_json::to_value(response)?)
            },
            HOST_UPDATE_VIEW => {
                let request: UpdateViewRequest = serde_json::from_value(params)?;
                map_api_result(
                    self.host
                        .update_view_handle(request.handle_id, request.view),
                )?;
                Ok(serde_json::Value::Null)
            },
            HOST_UPDATE_ITEMS => {
                let request: UpdateItemsRequest = serde_json::from_value(params)?;
                let items: RVec<_> = request.items.into();
                map_api_result(self.host.update_items_handle(request.handle_id, items))?;
                Ok(serde_json::Value::Null)
            },
            HOST_SET_LOADING => {
                let request: SetLoadingRequest = serde_json::from_value(params)?;
                map_api_result(
                    self.host
                        .set_loading_handle(request.handle_id, request.loading),
                )?;
                Ok(serde_json::Value::Null)
            },
            HOST_SET_ERROR => {
                let request: SetErrorRequest = serde_json::from_value(params)?;
                map_api_result(self.host.set_error_handle(request.handle_id, request.error))?;
                Ok(serde_json::Value::Null)
            },
            HOST_SHOW_TOAST => {
                let request: ToastRequest = serde_json::from_value(params)?;
                let toast = toast_from_payload(request.toast);
                map_api_result(self.host.show_toast(toast))?;
                Ok(serde_json::Value::Null)
            },
            HOST_SHOW_HUD => {
                let request: HudRequest = serde_json::from_value(params)?;
                map_api_result(
                    self.host
                        .show_hud(photoncast_extension_api::RStr::from_str(&request.message)),
                )?;
                Ok(serde_json::Value::Null)
            },
            HOST_COPY_CLIPBOARD => {
                let request: ClipboardCopyRequest = serde_json::from_value(params)?;
                map_api_result(
                    self.host
                        .copy_to_clipboard(photoncast_extension_api::RStr::from_str(&request.text)),
                )?;
                Ok(serde_json::Value::Null)
            },
            HOST_READ_CLIPBOARD => {
                let text = map_api_result(self.host.read_clipboard())?;
                let response = TextResponse {
                    text: text
                        .into_option()
                        .map(photoncast_extension_api::RString::into_string),
                };
                Ok(serde_json::to_value(response)?)
            },
            HOST_SELECTED_TEXT => {
                let text = map_api_result(self.host.selected_text())?;
                let response = TextResponse {
                    text: text
                        .into_option()
                        .map(photoncast_extension_api::RString::into_string),
                };
                Ok(serde_json::to_value(response)?)
            },
            HOST_OPEN_URL => {
                let request: OpenUrlRequest = serde_json::from_value(params)?;
                map_api_result(
                    self.host
                        .open_url(photoncast_extension_api::RStr::from_str(&request.url)),
                )?;
                Ok(serde_json::Value::Null)
            },
            HOST_OPEN_FILE => {
                let request: OpenPathRequest = serde_json::from_value(params)?;
                map_api_result(
                    self.host
                        .open_file(photoncast_extension_api::RStr::from_str(&request.path)),
                )?;
                Ok(serde_json::Value::Null)
            },
            HOST_REVEAL_IN_FINDER => {
                let request: OpenPathRequest = serde_json::from_value(params)?;
                map_api_result(
                    self.host
                        .reveal_in_finder(photoncast_extension_api::RStr::from_str(&request.path)),
                )?;
                Ok(serde_json::Value::Null)
            },
            HOST_LAUNCH_COMMAND => {
                let request: LaunchCommandRequest = serde_json::from_value(params)?;
                let args = ipc_args_to_api(request.args)?;
                let result = map_api_result(self.host.launch_command(
                    photoncast_extension_api::RStr::from_str(&request.extension_id),
                    photoncast_extension_api::RStr::from_str(&request.command_id),
                    args,
                ))?;
                let response = CommandResponse {
                    success: result.success,
                    message: result
                        .message
                        .into_option()
                        .map(photoncast_extension_api::RString::into_string),
                };
                Ok(serde_json::to_value(response)?)
            },
            HOST_GET_FRONTMOST_APP => {
                let app = map_api_result(self.host.get_frontmost_application())?;
                let response = FrontmostApplicationResponse {
                    application: app.into_option().map(map_frontmost_application),
                };
                Ok(serde_json::to_value(response)?)
            },
            _ => Err(IpcError::InvalidMessage(format!(
                "unknown method: {method}"
            ))),
        }
    }
}

impl RpcHandler for HostRpcHandler {
    fn handle_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, IpcError> {
        self.handle_method(method, params)
    }

    fn handle_notification(&self, method: &str, params: serde_json::Value) -> Result<(), IpcError> {
        let _ = self.handle_method(method, params)?;
        Ok(())
    }
}

pub fn spawn_sandboxed_extension(
    manifest: &ExtensionManifest,
    entry_path: &Path,
    host: ExtensionHostImpl,
) -> Result<SandboxedExtension, SandboxError> {
    let runner_path = resolve_runner_path()?;
    let manifest_dir =
        manifest
            .directory
            .as_ref()
            .ok_or_else(|| SandboxError::MissingManifestDir {
                extension_id: manifest.extension.id.clone(),
            })?;
    let manifest_path = manifest_dir.join("extension.toml");

    let mut command = Command::new(runner_path);
    command
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--entry")
        .arg(entry_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = command.spawn()?;
    let stdout = child.stdout.take().ok_or(SandboxError::MissingStdio)?;
    let stdin = child.stdin.take().ok_or(SandboxError::MissingStdio)?;

    let handler = Arc::new(HostRpcHandler::new(host));
    let connection = RpcConnection::new(BufReader::new(stdout), stdin, handler);

    Ok(SandboxedExtension {
        process: child,
        connection,
    })
}

fn resolve_runner_path() -> Result<PathBuf, SandboxError> {
    if let Ok(path) = std::env::var("PHOTONCAST_EXTENSION_RUNNER") {
        return Ok(PathBuf::from(path));
    }

    let current_exe = std::env::current_exe()?;
    let Some(dir) = current_exe.parent() else {
        return Err(SandboxError::RunnerNotFound { path: current_exe });
    };

    let candidate = dir.join("photoncast-extension-runner");
    if candidate.exists() {
        return Ok(candidate);
    }

    Err(SandboxError::RunnerNotFound { path: candidate })
}

fn map_api_result<T>(result: ExtensionApiResult<T>) -> Result<T, IpcError> {
    result.into_result().map_err(|err| IpcError::RpcError {
        code: -32000,
        message: err.to_string(),
    })
}

fn ipc_args_to_api(
    args: Option<CommandArguments>,
) -> Result<ROption<ApiCommandArguments>, IpcError> {
    let Some(args) = args else {
        return Ok(ROption::RNone);
    };

    let extra = match args.extra {
        Some(value) => {
            let json = serde_json::to_string(&value)?;
            let boxed = RawValueBox::try_from_string(json)
                .map_err(|err| IpcError::InvalidMessage(err.to_string()))?;
            ROption::RSome(boxed)
        },
        None => ROption::RNone,
    };

    Ok(ROption::RSome(ApiCommandArguments {
        query: args.query.map(RString::from).into(),
        selection: args.selection.map(RString::from).into(),
        clipboard: args.clipboard.map(RString::from).into(),
        extra,
    }))
}

fn toast_from_payload(payload: ToastPayload) -> Toast {
    let style = match payload.style {
        ToastStylePayload::Success => ToastStyle::Success,
        ToastStylePayload::Failure => ToastStyle::Failure,
        ToastStylePayload::Default => ToastStyle::Default,
    };

    Toast {
        style,
        title: RString::from(payload.title),
        message: payload.message.map(RString::from).into(),
    }
}

fn map_frontmost_application(app: ApplicationInfo) -> FrontmostApplicationInfo {
    FrontmostApplicationInfo {
        name: app.name.into_string(),
        bundle_id: app
            .bundle_id
            .into_option()
            .map(photoncast_extension_api::RString::into_string),
        path: app.path.into_string(),
    }
}
