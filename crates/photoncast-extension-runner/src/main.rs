use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use abi_stable::external_types::RawValueBox;
use abi_stable::std_types::{RBox, ROption, RString, RVec};
use photoncast_core::extensions::cache::ExtensionCache;
use photoncast_core::extensions::loader::ExtensionLoader;
use photoncast_core::extensions::manifest::{load_manifest, ExtensionManifest};
use photoncast_core::extensions::storage::{ExtensionStorageImpl, PreferenceStoreImpl};
use photoncast_core::utils::paths;
use photoncast_extension_api::{
    Action, ActionHandler, ApplicationInfo, CommandArguments as ApiCommandArguments,
    CommandInvocationResult, ExtensionApiError, ExtensionApiResult, ExtensionBox,
    ExtensionContext, ExtensionHost, ExtensionHostProtocol, ExtensionRuntime,
    ExtensionRuntimeTrait, ExtensionStorage, ExtensionView, PreferenceValues, Toast, ToastStyle,
    ViewHandle, ViewHandleTrait,
};
use photoncast_extension_ipc::messages::{
    ClipboardCopyRequest, CommandArguments as IpcCommandArguments, CommandRequest, CommandResponse,
    FrontmostApplicationInfo, FrontmostApplicationResponse, HudRequest, LaunchCommandRequest,
    OpenPathRequest, OpenUrlRequest, RenderViewRequest, RenderViewResponse, SearchItem,
    SearchRequest, SearchResponse, SetErrorRequest, SetLoadingRequest, TextResponse, ToastPayload,
    ToastRequest, ToastStylePayload, UpdateItemsRequest, UpdateViewRequest,
};
use photoncast_extension_ipc::methods::*;
use photoncast_extension_ipc::{IpcError, RpcConnection, RpcHandler};
use serde_json::Value;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (manifest_path, entry_path) = parse_args()?;
    let manifest = load_manifest(&manifest_path).map_err(|err| err.to_string())?;

    let runtime = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
    let runner_runtime = RunnerRuntime::new(runtime.handle().clone());

    let preference_store = PreferenceStoreImpl::new(manifest.preferences.clone());
    let storage = ExtensionStorageImpl::new(
        paths::data_dir().join("extensions_storage.db"),
        manifest.extension.id.clone(),
    )
    .map_err(|err| err.to_string())?;

    let cache = ExtensionCache::new(
        manifest.extension.id.clone(),
        paths::cache_dir()
            .join("extensions")
            .join(&manifest.extension.id),
    );

    let connection_cell = Arc::new(Mutex::new(None));
    let host = IpcExtensionHost::new(
        connection_cell.clone(),
        preference_store.clone(),
        storage.clone(),
    );
    let context_factory = ExtensionContextFactory::new(
        &manifest,
        host.clone(),
        runner_runtime.clone(),
        cache,
        preference_store,
        storage,
    )?;

    let library = ExtensionLoader::load(&entry_path).map_err(|err| err.to_string())?;
    let api_version =
        ExtensionLoader::resolve_api_version(library.raw()).map_err(|err| err.to_string())?;
    ExtensionLoader::check_api_version(api_version).map_err(|err| err.to_string())?;
    let root_module = ExtensionLoader::load_root_module(&library).map_err(|err| err.to_string())?;
    let instance = root_module.instantiate_extension();

    let shutdown = Arc::new(AtomicBool::new(false));
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let state = Arc::new(RunnerState {
        instance: Mutex::new(instance),
        context_factory,
        shutdown: shutdown.clone(),
        shutdown_tx,
    });

    let handler = Arc::new(ExtensionRpcHandler::new(state.clone()));
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let connection = RpcConnection::new(BufReader::new(stdin), stdout, handler);
    *connection_cell
        .lock()
        .map_err(|_| "connection lock poisoned")? = Some(connection.clone());

    activate_extension(&state)?;

    wait_for_shutdown(shutdown_rx, state.shutdown.clone());
    drop(runtime);
    Ok(())
}

fn parse_args() -> Result<(PathBuf, PathBuf), String> {
    let mut manifest = None;
    let mut entry = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => {
                manifest = args.next().map(PathBuf::from);
            },
            "--entry" => {
                entry = args.next().map(PathBuf::from);
            },
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    match (manifest, entry) {
        (Some(manifest), Some(entry)) => Ok((manifest, entry)),
        _ => Err("Usage: photoncast-extension-runner --manifest <path> --entry <path>".to_string()),
    }
}

fn activate_extension(state: &RunnerState) -> Result<(), String> {
    let ctx = state.context_factory.make_context();
    let mut instance = state
        .instance
        .lock()
        .map_err(|_| "extension lock poisoned")?;
    instance
        .activate(ctx)
        .into_result()
        .map_err(|err| err.to_string())?;

    let startup_ctx = state.context_factory.make_context();
    if let Err(err) = instance.on_startup(&startup_ctx).into_result() {
        eprintln!("Extension on_startup failed: {err}");
    }
    Ok(())
}

fn wait_for_shutdown(rx: mpsc::Receiver<()>, shutdown: Arc<AtomicBool>) {
    // Wait indefinitely for shutdown signal from host
    let _ = rx.recv();
    shutdown.store(true, Ordering::SeqCst);
}

#[derive(Clone)]
struct RunnerRuntime {
    handle: tokio::runtime::Handle,
}

impl RunnerRuntime {
    fn new(handle: tokio::runtime::Handle) -> Self {
        Self { handle }
    }
}

impl ExtensionRuntimeTrait for RunnerRuntime {
    fn spawn(&self, future: photoncast_extension_api::ExtensionFuture_TO<'static, RBox<()>>) {
        self.handle.spawn(async move {
            future.poll();
        });
    }
}

struct ExtensionContextFactory {
    extension_id: RString,
    data_dir: RString,
    cache_dir: RString,
    assets_dir: RString,
    app_version: RString,
    preference_store: PreferenceStoreImpl,
    storage: ExtensionStorageImpl,
    host: IpcExtensionHost,
    runtime: RunnerRuntime,
    cache: ExtensionCache,
}

impl ExtensionContextFactory {
    fn new(
        manifest: &ExtensionManifest,
        host: IpcExtensionHost,
        runtime: RunnerRuntime,
        cache: ExtensionCache,
        preference_store: PreferenceStoreImpl,
        storage: ExtensionStorageImpl,
    ) -> Result<Self, String> {
        let extension_data_dir = paths::data_dir()
            .join("extensions")
            .join(&manifest.extension.id);
        let extension_cache_dir = paths::cache_dir()
            .join("extensions")
            .join(&manifest.extension.id);
        let extension_assets_dir = extension_data_dir.join("assets");

        std::fs::create_dir_all(&extension_data_dir).map_err(|err| err.to_string())?;
        std::fs::create_dir_all(&extension_cache_dir).map_err(|err| err.to_string())?;
        std::fs::create_dir_all(&extension_assets_dir).map_err(|err| err.to_string())?;

        Ok(Self {
            extension_id: RString::from(manifest.extension.id.clone()),
            data_dir: RString::from(extension_data_dir.to_string_lossy().as_ref()),
            cache_dir: RString::from(extension_cache_dir.to_string_lossy().as_ref()),
            assets_dir: RString::from(extension_assets_dir.to_string_lossy().as_ref()),
            app_version: RString::from(env!("CARGO_PKG_VERSION")),
            preference_store,
            storage,
            host,
            runtime,
            cache,
        })
    }

    fn make_context(&self) -> ExtensionContext {
        ExtensionContext {
            data_dir: self.data_dir.clone(),
            cache_dir: self.cache_dir.clone(),
            preferences: self.preference_store.api_handle(),
            storage: self.storage.api_handle(),
            host: ExtensionHost::new(self.host.clone()),
            runtime: ExtensionRuntime::new(self.runtime.clone()),
            cache: self.cache.api_handle(),
            extension_id: self.extension_id.clone(),
            app_version: self.app_version.clone(),
            assets_dir: self.assets_dir.clone(),
        }
    }
}

struct RunnerState {
    instance: Mutex<ExtensionBox>,
    context_factory: ExtensionContextFactory,
    shutdown: Arc<AtomicBool>,
    shutdown_tx: mpsc::Sender<()>,
}

struct ExtensionRpcHandler {
    state: Arc<RunnerState>,
}

impl ExtensionRpcHandler {
    fn new(state: Arc<RunnerState>) -> Self {
        Self { state }
    }

    fn handle_search(&self, request: SearchRequest) -> Result<Value, IpcError> {
        let provider = {
            let instance = self
                .state
                .instance
                .lock()
                .map_err(|_| IpcError::Disconnected)?;
            instance.search_provider().into_option()
        };

        let mut items = Vec::new();
        if let Some(provider) = provider {
            let results = provider.search(RString::from(request.query), request.max_results);
            for item in results {
                items.push(SearchItem {
                    id: item.id.into_string(),
                    title: item.title.into_string(),
                    subtitle: item.subtitle.into_option().map(|value| value.into_string()),
                    icon: item.icon,
                    score: item.score,
                    actions: filter_actions(item.actions),
                });
            }
        }

        serde_json::to_value(SearchResponse { items }).map_err(IpcError::from)
    }

    fn handle_command(&self, request: CommandRequest) -> Result<Value, IpcError> {
        let command = {
            let instance = self
                .state
                .instance
                .lock()
                .map_err(|_| IpcError::Disconnected)?;
            instance
                .commands()
                .into_iter()
                .find(|command| command.id.as_str() == request.command_id)
        };

        let Some(command) = command else {
            let response = CommandResponse {
                success: false,
                message: Some("command not found".to_string()),
            };
            return serde_json::to_value(response).map_err(IpcError::from);
        };

        let args = ipc_args_to_api(request.args)?;
        let ctx = self.state.context_factory.make_context();
        let result = command.handler.handle(ctx, args).into_result();

        let response = match result {
            Ok(()) => CommandResponse {
                success: true,
                message: None,
            },
            Err(err) => CommandResponse {
                success: false,
                message: Some(err.to_string()),
            },
        };

        serde_json::to_value(response).map_err(IpcError::from)
    }

    fn handle_shutdown(&self) -> Result<Value, IpcError> {
        let mut instance = self
            .state
            .instance
            .lock()
            .map_err(|_| IpcError::Disconnected)?;
        let _ = map_api_result(instance.deactivate());
        self.state.shutdown.store(true, Ordering::SeqCst);
        let _ = self.state.shutdown_tx.send(());
        Ok(Value::Null)
    }
}

impl RpcHandler for ExtensionRpcHandler {
    fn handle_request(&self, method: &str, params: Value) -> Result<Value, IpcError> {
        match method {
            EXTENSION_SEARCH => {
                let request: SearchRequest = serde_json::from_value(params)?;
                self.handle_search(request)
            },
            EXTENSION_COMMAND => {
                let request: CommandRequest = serde_json::from_value(params)?;
                self.handle_command(request)
            },
            EXTENSION_SHUTDOWN => self.handle_shutdown(),
            _ => Err(IpcError::InvalidMessage(format!(
                "unknown method: {method}"
            ))),
        }
    }

    fn handle_notification(&self, method: &str, params: Value) -> Result<(), IpcError> {
        let _ = self.handle_request(method, params)?;
        Ok(())
    }
}

#[derive(Clone)]
struct IpcExtensionHost {
    connection: Arc<Mutex<Option<RpcConnection>>>,
    preference_store: PreferenceStoreImpl,
    storage: ExtensionStorageImpl,
}

impl IpcExtensionHost {
    fn new(
        connection: Arc<Mutex<Option<RpcConnection>>>,
        preference_store: PreferenceStoreImpl,
        storage: ExtensionStorageImpl,
    ) -> Self {
        Self {
            connection,
            preference_store,
            storage,
        }
    }

    fn connection(&self) -> Result<RpcConnection, ExtensionApiError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ExtensionApiError::message("connection lock poisoned"))?;
        connection
            .clone()
            .ok_or_else(|| ExtensionApiError::message("connection not initialized"))
    }

    fn send_request<T: serde::Serialize>(
        &self,
        method: &str,
        params: T,
    ) -> ExtensionApiResult<Value> {
        let params = match serde_json::to_value(params)
            .map_err(|err| ExtensionApiError::message(err.to_string()))
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let connection = match self.connection() {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        match connection.send_request(method, params) {
            Ok(value) => Ok(value).into(),
            Err(err) => Err(ExtensionApiError::message(err.to_string())).into(),
        }
    }

    #[allow(dead_code)]
    fn send_notification<T: serde::Serialize>(
        &self,
        method: &str,
        params: T,
    ) -> ExtensionApiResult<()> {
        let params = match serde_json::to_value(params)
            .map_err(|err| ExtensionApiError::message(err.to_string()))
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let connection = match self.connection() {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        match connection.send_notification(method, params) {
            Ok(()) => Ok(()).into(),
            Err(err) => Err(ExtensionApiError::message(err.to_string())).into(),
        }
    }
}

impl ExtensionHostProtocol for IpcExtensionHost {
    fn render_view(&self, view: ExtensionView) -> ExtensionApiResult<ViewHandle> {
        let response = match self
            .send_request(HOST_RENDER_VIEW, RenderViewRequest { view })
            .into_result()
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let response: RenderViewResponse = match serde_json::from_value(response)
            .map_err(|err| ExtensionApiError::message(err.to_string()))
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let handle = IpcViewHandle {
            connection: self.connection.clone(),
            handle_id: response.handle_id,
        };
        Ok(ViewHandle::new(handle)).into()
    }

    fn update_view(
        &self,
        _handle: ViewHandle,
        _patch: photoncast_extension_api::ViewPatch,
    ) -> ExtensionApiResult<()> {
        Ok(()).into()
    }

    fn show_toast(&self, toast: Toast) -> ExtensionApiResult<()> {
        let payload = ToastRequest {
            toast: toast_to_payload(toast),
        };
        if let Err(err) = self
            .send_request(HOST_SHOW_TOAST, payload)
            .into_result()
        {
            return Err(err).into();
        }
        Ok(()).into()
    }

    fn show_hud(&self, message: photoncast_extension_api::RStr<'_>) -> ExtensionApiResult<()> {
        let request = HudRequest {
            message: message.as_str().to_string(),
        };
        if let Err(err) = self.send_request(HOST_SHOW_HUD, request).into_result() {
            return Err(err).into();
        }
        Ok(()).into()
    }

    fn copy_to_clipboard(
        &self,
        text: photoncast_extension_api::RStr<'_>,
    ) -> ExtensionApiResult<()> {
        let request = ClipboardCopyRequest {
            text: text.as_str().to_string(),
        };
        if let Err(err) = self
            .send_request(HOST_COPY_CLIPBOARD, request)
            .into_result()
        {
            return Err(err).into();
        }
        Ok(()).into()
    }

    fn read_clipboard(&self) -> ExtensionApiResult<ROption<RString>> {
        let response = match self
            .send_request(HOST_READ_CLIPBOARD, Value::Null)
            .into_result()
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let response: TextResponse = match serde_json::from_value(response)
            .map_err(|err| ExtensionApiError::message(err.to_string()))
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        Ok(response.text.map(RString::from).into()).into()
    }

    fn selected_text(&self) -> ExtensionApiResult<ROption<RString>> {
        let response = match self
            .send_request(HOST_SELECTED_TEXT, Value::Null)
            .into_result()
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let response: TextResponse = match serde_json::from_value(response)
            .map_err(|err| ExtensionApiError::message(err.to_string()))
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        Ok(response.text.map(RString::from).into()).into()
    }

    fn open_url(&self, url: photoncast_extension_api::RStr<'_>) -> ExtensionApiResult<()> {
        let request = OpenUrlRequest {
            url: url.as_str().to_string(),
        };
        if let Err(err) = self.send_request(HOST_OPEN_URL, request).into_result() {
            return Err(err).into();
        }
        Ok(()).into()
    }

    fn open_file(&self, path: photoncast_extension_api::RStr<'_>) -> ExtensionApiResult<()> {
        let request = OpenPathRequest {
            path: path.as_str().to_string(),
        };
        if let Err(err) = self.send_request(HOST_OPEN_FILE, request).into_result() {
            return Err(err).into();
        }
        Ok(()).into()
    }

    fn reveal_in_finder(&self, path: photoncast_extension_api::RStr<'_>) -> ExtensionApiResult<()> {
        let request = OpenPathRequest {
            path: path.as_str().to_string(),
        };
        if let Err(err) = self
            .send_request(HOST_REVEAL_IN_FINDER, request)
            .into_result()
        {
            return Err(err).into();
        }
        Ok(()).into()
    }

    fn get_preferences(&self) -> ExtensionApiResult<PreferenceValues> {
        self.preference_store.values()
    }

    fn get_storage(&self) -> ExtensionApiResult<ExtensionStorage> {
        Ok(self.storage.api_handle()).into()
    }

    fn launch_command(
        &self,
        extension_id: photoncast_extension_api::RStr<'_>,
        command_id: photoncast_extension_api::RStr<'_>,
        args: ROption<ApiCommandArguments>,
    ) -> ExtensionApiResult<CommandInvocationResult> {
        let args = match api_args_to_ipc(args) {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let request = LaunchCommandRequest {
            extension_id: extension_id.as_str().to_string(),
            command_id: command_id.as_str().to_string(),
            args,
        };
        let response = match self
            .send_request(HOST_LAUNCH_COMMAND, request)
            .into_result()
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let response: CommandResponse = match serde_json::from_value(response)
            .map_err(|err| ExtensionApiError::message(err.to_string()))
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        Ok(CommandInvocationResult {
            success: response.success,
            message: response.message.map(RString::from).into(),
        })
        .into()
    }

    fn get_frontmost_application(&self) -> ExtensionApiResult<ROption<ApplicationInfo>> {
        let response = match self
            .send_request(HOST_GET_FRONTMOST_APP, Value::Null)
            .into_result()
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        let response: FrontmostApplicationResponse = match serde_json::from_value(response)
            .map_err(|err| ExtensionApiError::message(err.to_string()))
        {
            Ok(value) => value,
            Err(err) => return Err(err).into(),
        };
        Ok(response.application.map(map_frontmost_application).into()).into()
    }
}

#[derive(Clone)]
struct IpcViewHandle {
    connection: Arc<Mutex<Option<RpcConnection>>>,
    handle_id: u64,
}

impl IpcViewHandle {
    fn send_notification<T: serde::Serialize>(&self, method: &str, params: T) {
        let connection = self.connection.lock().ok().and_then(|guard| guard.clone());
        let Some(connection) = connection else {
            return;
        };
        let params = serde_json::to_value(params).unwrap_or(Value::Null);
        let _ = connection.send_notification(method, params);
    }
}

impl ViewHandleTrait for IpcViewHandle {
    fn update(&self, view: ExtensionView) {
        self.send_notification(
            HOST_UPDATE_VIEW,
            UpdateViewRequest {
                handle_id: self.handle_id,
                view,
            },
        );
    }

    fn update_items(&self, items: RVec<photoncast_extension_api::ListItem>) {
        let items = items.into_iter().collect();
        self.send_notification(
            HOST_UPDATE_ITEMS,
            UpdateItemsRequest {
                handle_id: self.handle_id,
                items,
            },
        );
    }

    fn set_loading(&self, loading: bool) {
        self.send_notification(
            HOST_SET_LOADING,
            SetLoadingRequest {
                handle_id: self.handle_id,
                loading,
            },
        );
    }

    fn set_error(&self, error: ROption<RString>) {
        let error = error
            .into_option()
            .map(photoncast_extension_api::RString::into_string);
        self.send_notification(
            HOST_SET_ERROR,
            SetErrorRequest {
                handle_id: self.handle_id,
                error,
            },
        );
    }
}

fn ipc_args_to_api(args: IpcCommandArguments) -> Result<ApiCommandArguments, IpcError> {
    let extra = match args.extra {
        Some(value) => {
            let json = serde_json::to_string(&value)?;
            let boxed = RawValueBox::try_from_string(json)
                .map_err(|err| IpcError::InvalidMessage(err.to_string()))?;
            ROption::RSome(boxed)
        },
        None => ROption::RNone,
    };

    Ok(ApiCommandArguments {
        query: args.query.map(RString::from).into(),
        selection: args.selection.map(RString::from).into(),
        clipboard: args.clipboard.map(RString::from).into(),
        extra,
    })
}

fn api_args_to_ipc(
    args: ROption<ApiCommandArguments>,
) -> Result<Option<IpcCommandArguments>, ExtensionApiError> {
    let Some(args) = args.into_option() else {
        return Ok(None);
    };

    let extra = match args
        .extra
        .into_option()
        .map(|value| serde_json::from_str::<Value>(value.get()))
        .transpose()
    {
        Ok(value) => value,
        Err(err) => return Err(ExtensionApiError::message(err.to_string())),
    };

    Ok(Some(IpcCommandArguments {
        query: args
            .query
            .into_option()
            .map(photoncast_extension_api::RString::into_string),
        selection: args
            .selection
            .into_option()
            .map(photoncast_extension_api::RString::into_string),
        clipboard: args
            .clipboard
            .into_option()
            .map(photoncast_extension_api::RString::into_string),
        extra,
    }))
}

fn toast_to_payload(toast: Toast) -> ToastPayload {
    let style = match toast.style {
        ToastStyle::Success => ToastStylePayload::Success,
        ToastStyle::Failure => ToastStylePayload::Failure,
        ToastStyle::Default => ToastStylePayload::Default,
    };

    ToastPayload {
        style,
        title: toast.title.into_string(),
        message: toast.message.into_option().map(|value| value.into_string()),
    }
}

fn map_frontmost_application(info: FrontmostApplicationInfo) -> ApplicationInfo {
    ApplicationInfo {
        name: RString::from(info.name),
        bundle_id: info.bundle_id.map(RString::from).into(),
        path: RString::from(info.path),
    }
}

fn filter_actions(actions: RVec<Action>) -> Vec<Action> {
    actions
        .into_iter()
        .filter(|action| !matches!(&action.handler, ActionHandler::Callback))
        .collect()
}

fn map_api_result<T>(
    result: photoncast_extension_api::ExtensionApiResult<T>,
) -> Result<T, IpcError> {
    result.into_result().map_err(|err| IpcError::RpcError {
        code: -32000,
        message: err.to_string(),
    })
}
