#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::option_if_let_else)]
#![allow(non_camel_case_types)]
// abi_stable's sabi_trait macro generates non-local impl blocks
#![allow(non_local_definitions)]
// abi_stable's sabi_trait macro generates unavoidable FFI pointer casts
#![allow(clippy::cast_ptr_alignment)]
// abi_stable's sabi_trait macro uses underscore-prefixed bindings internally
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::no_effect_underscore_binding)]
// abi_stable's sabi_trait macro generates explicit lifetimes that could be elided
#![allow(clippy::elidable_lifetime_names)]
// abi_stable's StableAbi derive macro generates explicit Clone on Copy types
#![allow(clippy::expl_impl_clone_on_copy)]

//! PhotonCast Extension API (ABI-stable).
//!
//! This crate defines the ABI-stable types and traits used by PhotonCast
//! extensions. It is shared by the host and extension dylibs.
//!
//! # ABI Stability and Version Coupling
//!
//! **CRITICAL**: The `abi_stable` crate provides a stable ABI between the host
//! application and dynamically loaded extension dylibs. This requires **exact
//! version matching** between:
//!
//! 1. The `abi_stable` version used by the host (`photoncast-core`)
//! 2. The `abi_stable` version used by extensions (`photoncast-extension-api`)
//!
//! ## Version Mismatch Consequences
//!
//! If versions don't match, the extension **will fail to load at runtime** with
//! an ABI incompatibility error. This is by design to prevent undefined behavior
//! from memory layout mismatches.
//!
//! ## Guidelines for Extension Authors
//!
//! - Always use the same `photoncast-extension-api` version as the host
//! - Do not override `abi_stable` version in your `Cargo.toml`
//! - When updating, rebuild all extensions against the new API version
//!
//! ## API Version
//!
//! The current API version is exported via [`ExtensionApiRootModule::api_version()`].
//! This is checked at extension load time to ensure compatibility.

use abi_stable::external_types::RawValueBox;
use abi_stable::sabi_trait;
use abi_stable::sabi_trait::prelude::TD_Opaque;
use abi_stable::std_types::{RBox, RDuration, RResult, RVec, Tuple2};

pub use abi_stable::std_types::{ROption, RStr, RString};
use abi_stable::StableAbi;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Serde helper module for `RVec<Tuple2<RString, RString>>`.
mod tuple2_vec_serde {
    use super::{Deserialize, Deserializer, RString, RVec, Serializer, Tuple2};

    pub fn serialize<S>(
        value: &RVec<Tuple2<RString, RString>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for item in value {
            seq.serialize_element(&(item.0.as_str(), item.1.as_str()))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RVec<Tuple2<RString, RString>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<(String, String)> = Vec::deserialize(deserializer)?;
        Ok(vec
            .into_iter()
            .map(|(k, v)| Tuple2(RString::from(k), RString::from(v)))
            .collect())
    }
}

pub mod prelude {
    pub use super::{
        Accessory, Action, ActionHandler, ActionStyle, Cache, CommandArguments, CommandHandler,
        CommandInvocationResult, CommandMode, EmptyState, Extension, ExtensionApiRootModule,
        ExtensionApiRootModule_Ref, ExtensionBox, ExtensionCommand, ExtensionContext,
        ExtensionHostProtocol, ExtensionSearchItem, ExtensionSearchProvider, ExtensionView,
        FieldType, FormField, FormView, GridItem, GridView, IconSource, ImageSource, ListItem,
        ListSection, ListView, MetadataItem, MetadataValue, Modifiers, PreferenceDefinition,
        PreferenceKind, PreferenceStore, PreferenceValue, PreferenceValues, Preview,
        SearchBarConfig, Shortcut, SubmitButton, TagColor, ViewHandle,
    };
}

pub const EXTENSION_API_VERSION: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Error, StableAbi)]
pub enum ExtensionApiError {
    #[error("extension error: {message}")]
    Message { message: RString },
}

impl ExtensionApiError {
    #[must_use]
    pub fn message(message: impl Into<RString>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

pub type ExtensionApiResult<T> = RResult<T, ExtensionApiError>;

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct ExtensionManifest {
    pub id: RString,
    pub name: RString,
    pub version: RString,
    pub description: ROption<RString>,
    pub author: ROption<RString>,
    pub license: ROption<RString>,
    pub homepage: ROption<RString>,
    pub min_photoncast_version: ROption<RString>,
    pub api_version: u32,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ExtensionContext {
    pub data_dir: RString,
    pub cache_dir: RString,
    pub preferences: PreferenceStore,
    pub storage: ExtensionStorage,
    pub host: ExtensionHost,
    pub runtime: ExtensionRuntime,
    pub cache: Cache,
    pub extension_id: RString,
    pub app_version: RString,
    pub assets_dir: RString,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ExtensionHost {
    inner: ExtensionHostProtocol_TO<'static, RBox<()>>,
}

impl ExtensionHost {
    #[must_use]
    pub fn new<T>(inner: T) -> Self
    where
        T: ExtensionHostProtocol + 'static,
    {
        Self {
            inner: ExtensionHostProtocol_TO::from_value(inner, TD_Opaque),
        }
    }

    pub fn show_toast(&self, toast: Toast) -> ExtensionApiResult<()> {
        self.inner.show_toast(toast)
    }

    pub fn show_hud(&self, message: impl AsRef<str>) -> ExtensionApiResult<()> {
        self.inner.show_hud(RStr::from_str(message.as_ref()))
    }

    pub fn copy_to_clipboard(&self, text: impl AsRef<str>) -> ExtensionApiResult<()> {
        self.inner.copy_to_clipboard(RStr::from_str(text.as_ref()))
    }

    pub fn read_clipboard(&self) -> ExtensionApiResult<ROption<RString>> {
        self.inner.read_clipboard()
    }

    pub fn selected_text(&self) -> ExtensionApiResult<ROption<RString>> {
        self.inner.selected_text()
    }

    pub fn open_url(&self, url: impl AsRef<str>) -> ExtensionApiResult<()> {
        self.inner.open_url(RStr::from_str(url.as_ref()))
    }

    pub fn open_file(&self, path: impl AsRef<str>) -> ExtensionApiResult<()> {
        self.inner.open_file(RStr::from_str(path.as_ref()))
    }

    pub fn reveal_in_finder(&self, path: impl AsRef<str>) -> ExtensionApiResult<()> {
        self.inner.reveal_in_finder(RStr::from_str(path.as_ref()))
    }

    pub fn render_view(&self, view: ExtensionView) -> ExtensionApiResult<ViewHandle> {
        self.inner.render_view(view)
    }

    pub fn update_view(&self, handle: ViewHandle, patch: ViewPatch) -> ExtensionApiResult<()> {
        self.inner.update_view(handle, patch)
    }

    pub fn get_preferences(&self) -> ExtensionApiResult<PreferenceValues> {
        self.inner.get_preferences()
    }

    pub fn get_storage(&self) -> ExtensionApiResult<ExtensionStorage> {
        self.inner.get_storage()
    }

    pub fn launch_command(
        &self,
        extension_id: RStr<'_>,
        command_id: RStr<'_>,
        args: ROption<CommandArguments>,
    ) -> ExtensionApiResult<CommandInvocationResult> {
        self.inner.launch_command(extension_id, command_id, args)
    }

    pub fn get_frontmost_application(&self) -> ExtensionApiResult<ROption<ApplicationInfo>> {
        self.inner.get_frontmost_application()
    }
}

#[sabi_trait]
pub trait ExtensionHostProtocol: Send + Sync {
    fn render_view(&self, view: ExtensionView) -> ExtensionApiResult<ViewHandle>;
    fn update_view(&self, handle: ViewHandle, patch: ViewPatch) -> ExtensionApiResult<()>;
    fn show_toast(&self, toast: Toast) -> ExtensionApiResult<()>;
    fn show_hud(&self, message: RStr<'_>) -> ExtensionApiResult<()>;
    fn copy_to_clipboard(&self, text: RStr<'_>) -> ExtensionApiResult<()>;
    fn read_clipboard(&self) -> ExtensionApiResult<ROption<RString>>;
    fn selected_text(&self) -> ExtensionApiResult<ROption<RString>>;
    fn open_url(&self, url: RStr<'_>) -> ExtensionApiResult<()>;
    fn open_file(&self, path: RStr<'_>) -> ExtensionApiResult<()>;
    fn reveal_in_finder(&self, path: RStr<'_>) -> ExtensionApiResult<()>;
    fn get_preferences(&self) -> ExtensionApiResult<PreferenceValues>;
    fn get_storage(&self) -> ExtensionApiResult<ExtensionStorage>;
    fn launch_command(
        &self,
        extension_id: RStr<'_>,
        command_id: RStr<'_>,
        args: ROption<CommandArguments>,
    ) -> ExtensionApiResult<CommandInvocationResult>;
    fn get_frontmost_application(&self) -> ExtensionApiResult<ROption<ApplicationInfo>>;
}

#[sabi_trait]
pub trait Extension: Send + Sync {
    fn manifest(&self) -> ExtensionManifest;
    fn activate(&mut self, ctx: ExtensionContext) -> ExtensionApiResult<()>;
    fn deactivate(&mut self) -> ExtensionApiResult<()>;
    /// Called after activation when permissions are granted.
    ///
    /// This hook is invoked:
    /// - When the app loads an extension (if permissions are already granted)
    /// - When permissions are first granted for an extension
    ///
    /// Use this for background tasks like pre-caching data, warming up caches,
    /// or other initialization that shouldn't block the main activation.
    fn on_startup(&mut self, _ctx: &ExtensionContext) -> ExtensionApiResult<()> {
        ExtensionApiResult::ROk(())
    }
    #[allow(clippy::option_if_let_else)]
    fn search_provider(&self) -> ROption<ExtensionSearchProvider_TO<'static, RBox<()>>> {
        ROption::RNone
    }
    #[allow(clippy::option_if_let_else)]
    fn commands(&self) -> RVec<ExtensionCommand> {
        RVec::new()
    }
}

#[sabi_trait]
pub trait ExtensionSearchProvider: Send + Sync {
    fn id(&self) -> RString;
    fn name(&self) -> RString;
    fn search(&self, query: RString, max_results: usize) -> RVec<ExtensionSearchItem>;
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ExtensionSearchItem {
    pub id: RString,
    pub title: RString,
    pub subtitle: ROption<RString>,
    pub icon: IconSource,
    pub score: f64,
    pub actions: RVec<Action>,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ExtensionCommand {
    pub id: RString,
    pub name: RString,
    pub mode: CommandMode,
    pub keywords: RVec<RString>,
    pub handler: CommandHandler,
    pub icon: ROption<IconSource>,
    pub subtitle: ROption<RString>,
    pub permissions: RVec<RString>,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi)]
pub enum CommandMode {
    Search,
    View,
    NoView,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct CommandHandler {
    inner: CommandHandlerTrait_TO<'static, RBox<()>>,
}

impl CommandHandler {
    #[must_use]
    pub fn new<T>(inner: T) -> Self
    where
        T: CommandHandlerTrait + 'static,
    {
        Self {
            inner: CommandHandlerTrait_TO::from_value(inner, TD_Opaque),
        }
    }

    pub fn handle(&self, ctx: ExtensionContext, args: CommandArguments) -> ExtensionApiResult<()> {
        self.inner.handle(ctx, args)
    }
}

#[sabi_trait]
pub trait CommandHandlerTrait: Send + Sync {
    fn handle(&self, ctx: ExtensionContext, args: CommandArguments) -> ExtensionApiResult<()>;
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct CommandArguments {
    pub query: ROption<RString>,
    pub selection: ROption<RString>,
    pub clipboard: ROption<RString>,
    pub extra: ROption<RawValueBox>,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct CommandInvocationResult {
    pub success: bool,
    pub message: ROption<RString>,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum ExtensionView {
    List(ListView),
    Detail(DetailView),
    Form(FormView),
    Grid(GridView),
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct ListView {
    pub title: RString,
    pub search_bar: ROption<SearchBarConfig>,
    pub sections: RVec<ListSection>,
    pub empty_state: ROption<EmptyState>,
    pub show_preview: bool,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct SearchBarConfig {
    pub placeholder: RString,
    pub throttle_ms: u32,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct ListSection {
    pub title: ROption<RString>,
    pub items: RVec<ListItem>,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct ListItem {
    pub id: RString,
    pub title: RString,
    pub subtitle: ROption<RString>,
    pub icon: IconSource,
    pub accessories: RVec<Accessory>,
    pub actions: RVec<Action>,
    pub preview: ROption<Preview>,
    pub shortcut: ROption<RString>,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum Accessory {
    Text(RString),
    Tag { text: RString, color: TagColor },
    Date(RDuration),
    Icon(IconSource),
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum Preview {
    Markdown(RString),
    Image {
        source: RString,
        alt: RString,
    },
    Metadata {
        #[serde(with = "tuple2_vec_serde")]
        items: RVec<Tuple2<RString, RString>>,
    },
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct EmptyState {
    pub icon: ROption<IconSource>,
    pub title: RString,
    pub description: ROption<RString>,
    pub actions: RVec<Action>,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct DetailView {
    pub title: RString,
    pub markdown: RString,
    pub metadata: RVec<MetadataItem>,
    pub actions: RVec<Action>,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct MetadataItem {
    pub label: RString,
    pub value: MetadataValue,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum MetadataValue {
    Text(RString),
    Link { text: RString, url: RString },
    Date(RDuration),
    Tag { text: RString, color: TagColor },
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct FormView {
    pub title: RString,
    pub description: ROption<RString>,
    pub fields: RVec<FormField>,
    pub submit: SubmitButton,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct FormField {
    pub id: RString,
    pub label: RString,
    pub field_type: FieldType,
    pub required: bool,
    pub placeholder: ROption<RString>,
    pub default_value: ROption<RString>,
    pub validation: ROption<Validation>,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum FieldType {
    TextField,
    TextArea {
        rows: u32,
    },
    Password,
    Number {
        min: ROption<f64>,
        max: ROption<f64>,
    },
    Checkbox,
    Dropdown {
        options: RVec<DropdownOption>,
    },
    FilePicker {
        allowed_extensions: RVec<RString>,
    },
    DirectoryPicker,
    DatePicker,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct DropdownOption {
    pub label: RString,
    pub value: RString,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct Validation {
    pub message: RString,
    pub pattern: ROption<RString>,
    pub min_length: ROption<u32>,
    pub max_length: ROption<u32>,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct SubmitButton {
    pub label: RString,
    pub shortcut: ROption<RString>,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct GridView {
    pub title: RString,
    pub columns: u32,
    pub items: RVec<GridItem>,
    pub empty_state: ROption<EmptyState>,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct GridItem {
    pub id: RString,
    pub title: RString,
    pub subtitle: ROption<RString>,
    pub image: ImageSource,
    pub actions: RVec<Action>,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum ImageSource {
    Path(RString),
    Url(RString),
    Base64 { data: RString, mime_type: RString },
    SfSymbol(RString),
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum IconSource {
    AppIcon {
        bundle_id: RString,
        icon_path: ROption<RString>,
    },
    SystemIcon {
        name: RString,
    },
    FileIcon {
        path: RString,
    },
    Emoji {
        glyph: RString,
    },
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct Action {
    pub id: RString,
    pub title: RString,
    pub icon: ROption<IconSource>,
    pub shortcut: ROption<Shortcut>,
    pub style: ActionStyle,
    pub handler: ActionHandler,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum ActionStyle {
    Default,
    Destructive,
    Primary,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum ActionHandler {
    /// Native callback handler - cannot be serialized for JSON-RPC transport.
    /// Actions with this handler type will be skipped during serialization.
    #[serde(skip)]
    Callback,
    OpenUrl(RString),
    OpenFile(RString),
    RevealInFinder(RString),
    QuickLook(RString),
    CopyToClipboard(RString),
    PushView(RBox<ExtensionView>),
    SubmitForm,
    MoveToTrash(RString),
    /// Copy an image file to clipboard (path to image file).
    CopyImageToClipboard(RString),
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: RString,
    pub modifiers: Modifiers,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub struct Modifiers {
    pub cmd: bool,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl Shortcut {
    #[must_use]
    pub fn cmd(key: impl Into<RString>) -> Self {
        Self {
            key: key.into(),
            modifiers: Modifiers {
                cmd: true,
                shift: false,
                alt: false,
                ctrl: false,
            },
        }
    }

    #[must_use]
    pub fn cmd_shift(key: impl Into<RString>) -> Self {
        Self {
            key: key.into(),
            modifiers: Modifiers {
                cmd: true,
                shift: true,
                alt: false,
                ctrl: false,
            },
        }
    }
}

impl Action {
    #[must_use]
    pub fn copy(text: impl Into<RString>) -> Self {
        Self {
            id: RString::from("copy"),
            title: RString::from("Copy"),
            icon: ROption::RNone,
            shortcut: ROption::RNone,
            style: ActionStyle::Default,
            handler: ActionHandler::CopyToClipboard(text.into()),
        }
    }

    #[must_use]
    pub fn open_url(url: impl Into<RString>) -> Self {
        Self {
            id: RString::from("open-url"),
            title: RString::from("Open URL"),
            icon: ROption::RNone,
            shortcut: ROption::RNone,
            style: ActionStyle::Default,
            handler: ActionHandler::OpenUrl(url.into()),
        }
    }

    #[must_use]
    pub fn open_file(path: impl Into<RString>) -> Self {
        Self {
            id: RString::from("open-file"),
            title: RString::from("Open File"),
            icon: ROption::RNone,
            shortcut: ROption::RNone,
            style: ActionStyle::Default,
            handler: ActionHandler::OpenFile(path.into()),
        }
    }

    #[must_use]
    pub fn reveal_in_finder(path: impl Into<RString>) -> Self {
        Self {
            id: RString::from("reveal-in-finder"),
            title: RString::from("Reveal in Finder"),
            icon: ROption::RNone,
            shortcut: ROption::RNone,
            style: ActionStyle::Default,
            handler: ActionHandler::RevealInFinder(path.into()),
        }
    }

    #[must_use]
    pub fn quick_look(path: impl Into<RString>) -> Self {
        Self {
            id: RString::from("quick-look"),
            title: RString::from("Quick Look"),
            icon: ROption::RNone,
            shortcut: ROption::RNone,
            style: ActionStyle::Default,
            handler: ActionHandler::QuickLook(path.into()),
        }
    }

    #[must_use]
    pub fn delete_with_confirmation(title: impl Into<RString>, handler: ActionHandler) -> Self {
        Self {
            id: RString::from("delete"),
            title: title.into(),
            icon: ROption::RNone,
            shortcut: ROption::RNone,
            style: ActionStyle::Destructive,
            handler,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi, Serialize, Deserialize)]
pub enum TagColor {
    Default,
    Blue,
    Green,
    Yellow,
    Orange,
    Red,
    Purple,
    Pink,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct ViewPatch {
    pub view: ExtensionView,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ViewHandle {
    inner: ViewHandleTrait_TO<'static, RBox<()>>,
}

impl ViewHandle {
    #[must_use]
    pub fn new<T>(inner: T) -> Self
    where
        T: ViewHandleTrait + 'static,
    {
        Self {
            inner: ViewHandleTrait_TO::from_value(inner, TD_Opaque),
        }
    }

    pub fn update(&self, view: ExtensionView) {
        self.inner.update(view);
    }

    pub fn update_items(&self, items: RVec<ListItem>) {
        self.inner.update_items(items);
    }

    pub fn set_loading(&self, loading: bool) {
        self.inner.set_loading(loading);
    }

    pub fn set_error(&self, error: ROption<RString>) {
        self.inner.set_error(error);
    }
}

#[sabi_trait]
pub trait ViewHandleTrait: Send + Sync {
    fn update(&self, view: ExtensionView);
    fn update_items(&self, items: RVec<ListItem>);
    fn set_loading(&self, loading: bool);
    fn set_error(&self, error: ROption<RString>);
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct Toast {
    pub style: ToastStyle,
    pub title: RString,
    pub message: ROption<RString>,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi)]
pub enum ToastStyle {
    Success,
    Failure,
    Default,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct ApplicationInfo {
    pub name: RString,
    pub bundle_id: ROption<RString>,
    pub path: RString,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct PreferenceStore {
    inner: PreferenceStoreTrait_TO<'static, RBox<()>>,
}

impl PreferenceStore {
    #[must_use]
    pub fn new<T>(inner: T) -> Self
    where
        T: PreferenceStoreTrait + 'static,
    {
        Self {
            inner: PreferenceStoreTrait_TO::from_value(inner, TD_Opaque),
        }
    }

    pub fn get(&self, key: RStr<'_>) -> ExtensionApiResult<ROption<PreferenceValue>> {
        self.inner.get(key)
    }

    pub fn set(&self, key: RStr<'_>, value: PreferenceValue) -> ExtensionApiResult<()> {
        self.inner.set(key, value)
    }

    pub fn definitions(&self) -> RVec<PreferenceDefinition> {
        self.inner.definitions()
    }
}

#[sabi_trait]
pub trait PreferenceStoreTrait: Send + Sync {
    fn get(&self, key: RStr<'_>) -> ExtensionApiResult<ROption<PreferenceValue>>;
    fn set(&self, key: RStr<'_>, value: PreferenceValue) -> ExtensionApiResult<()>;
    fn definitions(&self) -> RVec<PreferenceDefinition>;
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct PreferenceDefinition {
    pub name: RString,
    pub title: RString,
    pub description: ROption<RString>,
    pub required: bool,
    pub kind: PreferenceKind,
    pub default_value: ROption<PreferenceValue>,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi)]
pub enum PreferenceKind {
    String,
    Number,
    Boolean,
    Secret,
    Select { options: RVec<SelectOption> },
    File,
    Directory,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct SelectOption {
    pub label: RString,
    pub value: RString,
}

#[repr(u8)]
#[derive(Debug, Clone, StableAbi)]
pub enum PreferenceValue {
    String(RString),
    Number(f64),
    Boolean(bool),
    Secret(RString),
    Select(RString),
    File(RString),
    Directory(RString),
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct PreferenceValues {
    pub values: RVec<Tuple2<RString, PreferenceValue>>,
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ExtensionStorage {
    inner: ExtensionStorageTrait_TO<'static, RBox<()>>,
}

impl ExtensionStorage {
    #[must_use]
    pub fn new<T>(inner: T) -> Self
    where
        T: ExtensionStorageTrait + 'static,
    {
        Self {
            inner: ExtensionStorageTrait_TO::from_value(inner, TD_Opaque),
        }
    }

    pub fn get(&self, key: RStr<'_>) -> ExtensionApiResult<ROption<RString>> {
        self.inner.get(key)
    }

    pub fn set(&self, key: RStr<'_>, value: RStr<'_>) -> ExtensionApiResult<()> {
        self.inner.set(key, value)
    }

    pub fn delete(&self, key: RStr<'_>) -> ExtensionApiResult<()> {
        self.inner.delete(key)
    }

    pub fn list(&self) -> ExtensionApiResult<RVec<RString>> {
        self.inner.list()
    }
}

#[sabi_trait]
pub trait ExtensionStorageTrait: Send + Sync {
    fn get(&self, key: RStr<'_>) -> ExtensionApiResult<ROption<RString>>;
    fn set(&self, key: RStr<'_>, value: RStr<'_>) -> ExtensionApiResult<()>;
    fn delete(&self, key: RStr<'_>) -> ExtensionApiResult<()>;
    fn list(&self) -> ExtensionApiResult<RVec<RString>>;
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ExtensionRuntime {
    inner: ExtensionRuntimeTrait_TO<'static, RBox<()>>,
}

impl ExtensionRuntime {
    #[must_use]
    pub fn new<T>(inner: T) -> Self
    where
        T: ExtensionRuntimeTrait + 'static,
    {
        Self {
            inner: ExtensionRuntimeTrait_TO::from_value(inner, TD_Opaque),
        }
    }

    pub fn spawn(&self, future: ExtensionFuture_TO<'static, RBox<()>>) {
        self.inner.spawn(future);
    }
}

#[sabi_trait]
pub trait ExtensionRuntimeTrait: Send + Sync {
    fn spawn(&self, future: ExtensionFuture_TO<'static, RBox<()>>);
}

#[sabi_trait]
pub trait ExtensionFuture: Send + Sync {
    fn poll(&self);
}

#[repr(C)]
#[derive(StableAbi)]
pub struct Cache {
    inner: CacheTrait_TO<'static, RBox<()>>,
}

impl Cache {
    #[must_use]
    pub fn new<T>(inner: T) -> Self
    where
        T: CacheTrait + 'static,
    {
        Self {
            inner: CacheTrait_TO::from_value(inner, TD_Opaque),
        }
    }

    pub fn get(&self, key: RStr<'_>) -> ROption<RawValueBox> {
        self.inner.get(key)
    }

    pub fn set(&self, key: RStr<'_>, value: RawValueBox, ttl: ROption<RDuration>) {
        self.inner.set(key, value, ttl);
    }

    pub fn remove(&self, key: RStr<'_>) {
        self.inner.remove(key);
    }

    pub fn clear(&self) {
        self.inner.clear();
    }

    pub fn has(&self, key: RStr<'_>) -> bool {
        self.inner.has(key)
    }
}

#[sabi_trait]
pub trait CacheTrait: Send + Sync {
    fn get(&self, key: RStr<'_>) -> ROption<RawValueBox>;
    fn set(&self, key: RStr<'_>, value: RawValueBox, ttl: ROption<RDuration>);
    fn remove(&self, key: RStr<'_>);
    fn clear(&self);
    fn has(&self, key: RStr<'_>) -> bool;
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
pub struct CacheEntryMetadata {
    pub key: RString,
    pub expires_at: ROption<RDuration>,
    pub persisted: bool,
}

#[repr(C)]
#[derive(Debug, Clone, StableAbi)]
#[sabi(kind(Prefix(prefix_ref = ExtensionApiRootModule_Ref)))]
#[sabi(missing_field(panic))]
pub struct ExtensionApiRootModule {
    #[sabi(last_prefix_field)]
    pub create_extension: extern "C" fn() -> ExtensionBox,
}

impl ExtensionApiRootModule {
    #[must_use]
    pub const fn api_version() -> u32 {
        EXTENSION_API_VERSION
    }
}

impl ExtensionApiRootModule_Ref {
    #[must_use]
    pub fn instantiate_extension(&self) -> ExtensionBox {
        (self.create_extension())()
    }
}

impl abi_stable::library::RootModule for ExtensionApiRootModule_Ref {
    abi_stable::declare_root_module_statics! {ExtensionApiRootModule_Ref}
    const BASE_NAME: &'static str = "photoncast_extension";
    const NAME: &'static str = "photoncast_extension";
    const VERSION_STRINGS: abi_stable::sabi_types::VersionStrings =
        abi_stable::package_version_strings!();
}

pub type ExtensionBox = Extension_TO<'static, RBox<()>>;

#[no_mangle]
pub const extern "C" fn photoncast_extension_api_version() -> u32 {
    EXTENSION_API_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_view_serialization() {
        let view = ListView {
            title: RString::from("Test List"),
            search_bar: ROption::RSome(SearchBarConfig {
                placeholder: RString::from("Search..."),
                throttle_ms: 100,
            }),
            sections: RVec::from(vec![ListSection {
                title: ROption::RSome(RString::from("Section 1")),
                items: RVec::new(),
            }]),
            empty_state: ROption::RNone,
            show_preview: true,
        };

        let json = serde_json::to_string(&view).expect("Failed to serialize ListView");
        let parsed: ListView = serde_json::from_str(&json).expect("Failed to deserialize ListView");

        assert_eq!(view.title.as_str(), parsed.title.as_str());
        assert_eq!(view.show_preview, parsed.show_preview);
    }

    #[test]
    fn test_detail_view_serialization() {
        let view = DetailView {
            title: RString::from("Test Detail"),
            markdown: RString::from("# Hello\n\nThis is a test."),
            metadata: RVec::from(vec![MetadataItem {
                label: RString::from("Author"),
                value: MetadataValue::Text(RString::from("Test User")),
            }]),
            actions: RVec::new(),
        };

        let json = serde_json::to_string(&view).expect("Failed to serialize DetailView");
        let parsed: DetailView =
            serde_json::from_str(&json).expect("Failed to deserialize DetailView");

        assert_eq!(view.title.as_str(), parsed.title.as_str());
        assert_eq!(view.markdown.as_str(), parsed.markdown.as_str());
    }

    #[test]
    fn test_form_view_serialization() {
        let view = FormView {
            title: RString::from("Test Form"),
            description: ROption::RSome(RString::from("Fill out this form")),
            fields: RVec::from(vec![FormField {
                id: RString::from("name"),
                label: RString::from("Name"),
                field_type: FieldType::TextField,
                required: true,
                placeholder: ROption::RSome(RString::from("Enter your name")),
                default_value: ROption::RNone,
                validation: ROption::RNone,
            }]),
            submit: SubmitButton {
                label: RString::from("Submit"),
                shortcut: ROption::RNone,
            },
        };

        let json = serde_json::to_string(&view).expect("Failed to serialize FormView");
        let parsed: FormView = serde_json::from_str(&json).expect("Failed to deserialize FormView");

        assert_eq!(view.title.as_str(), parsed.title.as_str());
        assert_eq!(view.fields.len(), parsed.fields.len());
    }

    #[test]
    fn test_grid_view_serialization() {
        let view = GridView {
            title: RString::from("Test Grid"),
            columns: 4,
            items: RVec::from(vec![GridItem {
                id: RString::from("item-1"),
                title: RString::from("Item 1"),
                subtitle: ROption::RNone,
                image: ImageSource::SfSymbol(RString::from("star.fill")),
                actions: RVec::new(),
            }]),
            empty_state: ROption::RNone,
        };

        let json = serde_json::to_string(&view).expect("Failed to serialize GridView");
        let parsed: GridView = serde_json::from_str(&json).expect("Failed to deserialize GridView");

        assert_eq!(view.title.as_str(), parsed.title.as_str());
        assert_eq!(view.columns, parsed.columns);
    }

    #[test]
    fn test_action_serialization() {
        let action = Action {
            id: RString::from("copy-action"),
            title: RString::from("Copy"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("doc.on.doc"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("c")),
            style: ActionStyle::Default,
            handler: ActionHandler::CopyToClipboard(RString::from("test text")),
        };

        let json = serde_json::to_string(&action).expect("Failed to serialize Action");
        let parsed: Action = serde_json::from_str(&json).expect("Failed to deserialize Action");

        assert_eq!(action.id.as_str(), parsed.id.as_str());
        assert_eq!(action.title.as_str(), parsed.title.as_str());
    }

    #[test]
    fn test_action_handler_variants() {
        // Test OpenUrl
        let handler = ActionHandler::OpenUrl(RString::from("https://example.com"));
        let json = serde_json::to_string(&handler).expect("Failed to serialize OpenUrl");
        let parsed: ActionHandler =
            serde_json::from_str(&json).expect("Failed to deserialize OpenUrl");
        assert!(matches!(parsed, ActionHandler::OpenUrl(_)));

        // Test CopyToClipboard
        let handler = ActionHandler::CopyToClipboard(RString::from("copied text"));
        let json = serde_json::to_string(&handler).expect("Failed to serialize CopyToClipboard");
        let parsed: ActionHandler =
            serde_json::from_str(&json).expect("Failed to deserialize CopyToClipboard");
        assert!(matches!(parsed, ActionHandler::CopyToClipboard(_)));

        // Test SubmitForm
        let handler = ActionHandler::SubmitForm;
        let json = serde_json::to_string(&handler).expect("Failed to serialize SubmitForm");
        let parsed: ActionHandler =
            serde_json::from_str(&json).expect("Failed to deserialize SubmitForm");
        assert!(matches!(parsed, ActionHandler::SubmitForm));
    }

    #[test]
    fn test_preview_metadata_serialization() {
        let preview = Preview::Metadata {
            items: RVec::from(vec![
                Tuple2(RString::from("key1"), RString::from("value1")),
                Tuple2(RString::from("key2"), RString::from("value2")),
            ]),
        };

        let json = serde_json::to_string(&preview).expect("Failed to serialize Preview::Metadata");
        let parsed: Preview =
            serde_json::from_str(&json).expect("Failed to deserialize Preview::Metadata");

        if let Preview::Metadata { items } = parsed {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].0.as_str(), "key1");
            assert_eq!(items[0].1.as_str(), "value1");
        } else {
            panic!("Expected Preview::Metadata variant");
        }
    }

    #[test]
    fn test_extension_view_enum_serialization() {
        let list_view = ExtensionView::List(ListView {
            title: RString::from("Test"),
            search_bar: ROption::RNone,
            sections: RVec::new(),
            empty_state: ROption::RNone,
            show_preview: false,
        });

        let json =
            serde_json::to_string(&list_view).expect("Failed to serialize ExtensionView::List");
        let parsed: ExtensionView =
            serde_json::from_str(&json).expect("Failed to deserialize ExtensionView::List");

        assert!(matches!(parsed, ExtensionView::List(_)));
    }

    #[test]
    fn test_accessory_variants() {
        let text = Accessory::Text(RString::from("Label"));
        let json = serde_json::to_string(&text).expect("Failed to serialize Accessory::Text");
        let parsed: Accessory =
            serde_json::from_str(&json).expect("Failed to deserialize Accessory::Text");
        assert!(matches!(parsed, Accessory::Text(_)));

        let tag = Accessory::Tag {
            text: RString::from("Important"),
            color: TagColor::Red,
        };
        let json = serde_json::to_string(&tag).expect("Failed to serialize Accessory::Tag");
        let parsed: Accessory =
            serde_json::from_str(&json).expect("Failed to deserialize Accessory::Tag");
        assert!(matches!(parsed, Accessory::Tag { .. }));
    }
}
