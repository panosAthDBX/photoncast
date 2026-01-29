use photoncast_extension_api::{Action, ExtensionView, IconSource, ListItem};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub max_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: IconSource,
    pub score: f64,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub items: Vec<SearchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArguments {
    pub query: Option<String>,
    pub selection: Option<String>,
    pub clipboard: Option<String>,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command_id: String,
    pub args: CommandArguments,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderViewRequest {
    pub view: ExtensionView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderViewResponse {
    pub handle_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateViewRequest {
    pub handle_id: u64,
    pub view: ExtensionView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateItemsRequest {
    pub handle_id: u64,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLoadingRequest {
    pub handle_id: u64,
    pub loading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetErrorRequest {
    pub handle_id: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastRequest {
    pub toast: ToastPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastPayload {
    pub style: ToastStylePayload,
    pub title: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToastStylePayload {
    Success,
    Failure,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardCopyRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenUrlRequest {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchCommandRequest {
    pub extension_id: String,
    pub command_id: String,
    pub args: Option<CommandArguments>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResponse {
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontmostApplicationInfo {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontmostApplicationResponse {
    pub application: Option<FrontmostApplicationInfo>,
}
