use std::collections::HashMap;
use std::path::PathBuf;

use abi_stable::std_types::RVec;
use abi_stable::std_types::{ROption, RString};
use parking_lot::RwLock;
use photoncast_extension_api::RStr;
use photoncast_extension_api::{
    ApplicationInfo, ExtensionApiError, ExtensionApiResult, ExtensionHostProtocol, ExtensionView,
    PreferenceValues, Toast, ViewHandle,
};
use tracing::info;

use crate::extensions::api_bridge::{HostViewHandle, HostViewHandleApi};
use crate::extensions::manager::CommandInvocationGuard;
use crate::extensions::storage::{ExtensionStorageImpl, PreferenceStoreImpl};
use crate::platform;
use crate::utils::paths;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ExtensionHostImpl {
    pub view_handles: std::sync::Arc<RwLock<Vec<HostViewHandle>>>,
    pub view_handle_index: std::sync::Arc<RwLock<HashMap<u64, HostViewHandle>>>,
    pub services: Option<ExtensionHostServices>,
}

#[derive(Clone)]
pub struct ExtensionHostServices {
    pub preference_store: PreferenceStoreImpl,
    pub storage: Arc<Mutex<ExtensionStorageImpl>>,
    pub command_invocation_guard: CommandInvocationGuard,
    /// Allowed filesystem paths from the extension manifest.
    pub allowed_filesystem_paths: Vec<PathBuf>,
}

// ExtensionHostServices is automatically Send + Sync because all fields are:
// - PreferenceStoreImpl: Arc<RwLock<Vec<...>>> fields — Send + Sync
// - Arc<Mutex<ExtensionStorageImpl>>: Mutex<T>: Sync requires T: Send — satisfied
// - CommandInvocationGuard: Arc<RwLock<HashSet<String>>> — Send + Sync
// - Vec<PathBuf>: Send + Sync

impl ExtensionHostImpl {
    #[must_use]
    pub fn new() -> Self {
        Self {
            view_handles: std::sync::Arc::new(RwLock::new(Vec::new())),
            view_handle_index: std::sync::Arc::new(RwLock::new(HashMap::new())),
            services: None,
        }
    }

    #[must_use]
    pub fn with_services(services: ExtensionHostServices) -> Self {
        Self {
            view_handles: std::sync::Arc::new(RwLock::new(Vec::new())),
            view_handle_index: std::sync::Arc::new(RwLock::new(HashMap::new())),
            services: Some(services),
        }
    }
}

impl Default for ExtensionHostImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionHostImpl {
    pub fn render_view_handle(&self, view: ExtensionView) -> HostViewHandle {
        let handle = HostViewHandle::new();
        handle.update(view);
        let handle_id = handle.id().value();
        self.view_handles.write().push(handle.clone());
        self.view_handle_index
            .write()
            .insert(handle_id, handle.clone());
        handle
    }

    pub fn view_handle(&self, handle_id: u64) -> Option<HostViewHandle> {
        self.view_handle_index.read().get(&handle_id).cloned()
    }

    pub fn update_view_handle(
        &self,
        handle_id: u64,
        view: ExtensionView,
    ) -> ExtensionApiResult<()> {
        let Some(handle) = self.view_handle(handle_id) else {
            return Err(ExtensionApiError::message("view handle not found")).into();
        };
        handle.update(view);
        Ok(()).into()
    }

    pub fn update_items_handle(
        &self,
        handle_id: u64,
        items: RVec<photoncast_extension_api::ListItem>,
    ) -> ExtensionApiResult<()> {
        let Some(handle) = self.view_handle(handle_id) else {
            return Err(ExtensionApiError::message("view handle not found")).into();
        };
        handle.update_items(items);
        Ok(()).into()
    }

    pub fn set_loading_handle(&self, handle_id: u64, loading: bool) -> ExtensionApiResult<()> {
        let Some(handle) = self.view_handle(handle_id) else {
            return Err(ExtensionApiError::message("view handle not found")).into();
        };
        handle.set_loading(loading);
        Ok(()).into()
    }

    pub fn set_error_handle(
        &self,
        handle_id: u64,
        error: Option<String>,
    ) -> ExtensionApiResult<()> {
        let Some(handle) = self.view_handle(handle_id) else {
            return Err(ExtensionApiError::message("view handle not found")).into();
        };
        handle.set_error(error);
        Ok(()).into()
    }

    pub fn take_pending_view(&self) -> Option<ExtensionView> {
        let mut handles = self.view_handles.write();
        let handle = handles.pop()?;
        let handle_id = handle.id().value();
        // Also remove from index to prevent memory leak
        self.view_handle_index.write().remove(&handle_id);
        handle.view()
    }

    pub fn clear_view_handles(&self) {
        self.view_handles.write().clear();
        self.view_handle_index.write().clear();
    }

    /// Checks if a path is allowed by the declared filesystem permissions.
    /// Returns true if no services are configured (permissive mode) or if the path
    /// is under one of the allowed paths.
    fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        let Some(services) = &self.services else {
            // No services configured - permissive mode for testing
            return true;
        };

        if services.allowed_filesystem_paths.is_empty() {
            // No filesystem permissions declared - deny all
            return false;
        }

        // Canonicalize the target path if possible, otherwise use as-is
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        for allowed in &services.allowed_filesystem_paths {
            // Expand ~ to home directory
            let expanded = if allowed.starts_with("~") {
                if let Some(home) = dirs::home_dir() {
                    home.join(allowed.strip_prefix("~/").unwrap_or(allowed.as_path()))
                } else {
                    allowed.clone()
                }
            } else {
                allowed.clone()
            };

            // Canonicalize allowed path if possible
            let canonical_allowed = expanded.canonicalize().unwrap_or(expanded);

            // Check if target path is under the allowed path
            if canonical_path.starts_with(&canonical_allowed) {
                return true;
            }
        }

        false
    }
}

impl ExtensionHostProtocol for ExtensionHostImpl {
    fn render_view(&self, view: ExtensionView) -> ExtensionApiResult<ViewHandle> {
        let handle = self.render_view_handle(view);
        Ok(HostViewHandleApi::new(handle).into_view_handle()).into()
    }

    fn update_view(
        &self,
        handle: ViewHandle,
        _patch: photoncast_extension_api::ViewPatch,
    ) -> ExtensionApiResult<()> {
        let _ = handle; // Placeholder - actual patching occurs in UI layer
        Ok(()).into()
    }

    fn show_toast(&self, toast: Toast) -> ExtensionApiResult<()> {
        info!(title = %toast.title, "Extension toast");
        Ok(()).into()
    }

    fn show_hud(&self, message: RStr<'_>) -> ExtensionApiResult<()> {
        info!(message = %message.as_str(), "HUD message");
        Ok(()).into()
    }

    fn copy_to_clipboard(&self, text: RStr<'_>) -> ExtensionApiResult<()> {
        let child = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(child) => child,
            Err(e) => return Err(ExtensionApiError::message(e.to_string())).into(),
        };

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            if let Err(e) = stdin.write_all(text.as_str().as_bytes()) {
                return Err(ExtensionApiError::message(e.to_string())).into();
            }
        }

        match child.wait() {
            Ok(status) if status.success() => Ok(()).into(),
            Ok(_) => Err(ExtensionApiError::message("pbcopy failed")).into(),
            Err(e) => Err(ExtensionApiError::message(e.to_string())).into(),
        }
    }

    fn read_clipboard(&self) -> ExtensionApiResult<ROption<RString>> {
        let output = std::process::Command::new("pbpaste").output();
        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                Ok(ROption::RSome(RString::from(text))).into()
            },
            Ok(_) => Err(ExtensionApiError::message("pbpaste failed")).into(),
            Err(e) => Err(ExtensionApiError::message(e.to_string())).into(),
        }
    }

    fn selected_text(&self) -> ExtensionApiResult<ROption<RString>> {
        // TODO: Hook into accessibility APIs to read selected text
        Ok(ROption::RNone).into()
    }

    fn open_url(&self, url: RStr<'_>) -> ExtensionApiResult<()> {
        match platform::launch::open_url(url.as_str()) {
            Ok(()) => Ok(()).into(),
            Err(e) => Err(ExtensionApiError::message(e.to_string())).into(),
        }
    }

    fn open_file(&self, path: RStr<'_>) -> ExtensionApiResult<()> {
        let path_buf = PathBuf::from(path.as_str());
        if !self.is_path_allowed(&path_buf) {
            return Err(ExtensionApiError::message(format!(
                "Permission denied: '{}' is not in allowed filesystem paths",
                path.as_str()
            )))
            .into();
        }
        match platform::launch::open_file(&path_buf) {
            Ok(()) => Ok(()).into(),
            Err(e) => Err(ExtensionApiError::message(e.to_string())).into(),
        }
    }

    fn reveal_in_finder(&self, path: RStr<'_>) -> ExtensionApiResult<()> {
        let path_buf = PathBuf::from(path.as_str());
        if !self.is_path_allowed(&path_buf) {
            return Err(ExtensionApiError::message(format!(
                "Permission denied: '{}' is not in allowed filesystem paths",
                path.as_str()
            )))
            .into();
        }
        match platform::launch::reveal_in_finder(&path_buf) {
            Ok(()) => Ok(()).into(),
            Err(e) => Err(ExtensionApiError::message(e.to_string())).into(),
        }
    }

    fn get_preferences(&self) -> ExtensionApiResult<PreferenceValues> {
        if let Some(services) = &self.services {
            services.preference_store.values()
        } else {
            Ok(PreferenceValues {
                values: RVec::new(),
            })
            .into()
        }
    }

    fn get_storage(&self) -> ExtensionApiResult<photoncast_extension_api::ExtensionStorage> {
        if let Some(services) = &self.services {
            let storage = match services.storage.lock() {
                Ok(storage) => storage.api_handle(),
                Err(_) => return Err(ExtensionApiError::message("storage lock poisoned")).into(),
            };
            Ok(storage).into()
        } else {
            Err(ExtensionApiError::message("storage not available")).into()
        }
    }

    fn launch_command(
        &self,
        extension_id: RStr<'_>,
        command_id: RStr<'_>,
        _args: ROption<photoncast_extension_api::CommandArguments>,
    ) -> ExtensionApiResult<photoncast_extension_api::CommandInvocationResult> {
        let extension_id = extension_id.as_str();
        let command_id = command_id.as_str();
        if let Some(services) = &self.services {
            if services
                .command_invocation_guard
                .is_invocation_allowed(extension_id, command_id)
            {
                Ok(photoncast_extension_api::CommandInvocationResult {
                    success: true,
                    message: ROption::RNone,
                })
                .into()
            } else {
                Err(ExtensionApiError::message("circular invocation")).into()
            }
        } else {
            Err(ExtensionApiError::message("command invocation unavailable")).into()
        }
    }

    fn get_frontmost_application(&self) -> ExtensionApiResult<ROption<ApplicationInfo>> {
        #[cfg(target_os = "macos")]
        {
            use objc2_app_kit::NSWorkspace;
            let workspace = NSWorkspace::sharedWorkspace();
            if let Some(app) = workspace.frontmostApplication() {
                let name = app
                    .localizedName()
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let bundle_id = app.bundleIdentifier().map(|b| b.to_string());
                let path = app
                    .bundleURL()
                    .and_then(|url| url.path().map(|p| p.to_string()));
                let path_str = path.unwrap_or_else(|| paths::data_dir().to_string_lossy().into());
                return Ok(ROption::RSome(ApplicationInfo {
                    name: RString::from(name),
                    bundle_id: bundle_id.map(RString::from).into(),
                    path: RString::from(path_str),
                }))
                .into();
            }
        }
        Ok(ROption::RNone).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_stable::std_types::{ROption, RString, RVec};
    use photoncast_extension_api::{ExtensionView, ListView};

    fn create_test_view() -> ExtensionView {
        ExtensionView::List(ListView {
            title: RString::from("Test"),
            search_bar: ROption::RNone,
            sections: RVec::new(),
            empty_state: ROption::RNone,
            show_preview: false,
        })
    }

    #[test]
    fn test_render_view_handle_adds_to_both_collections() {
        let host = ExtensionHostImpl::new();
        let view = create_test_view();

        let handle = host.render_view_handle(view);
        let handle_id = handle.id().value();

        // Verify both collections contain the handle
        assert_eq!(host.view_handles.read().len(), 1);
        assert!(host.view_handle_index.read().contains_key(&handle_id));
    }

    #[test]
    fn test_take_pending_view_removes_from_both_collections() {
        let host = ExtensionHostImpl::new();
        let view = create_test_view();

        let handle = host.render_view_handle(view);
        let handle_id = handle.id().value();

        // Take the pending view
        let result = host.take_pending_view();
        assert!(result.is_some());

        // Verify both collections are now empty
        assert!(host.view_handles.read().is_empty());
        assert!(!host.view_handle_index.read().contains_key(&handle_id));
    }

    #[test]
    fn test_take_pending_view_returns_none_when_empty() {
        let host = ExtensionHostImpl::new();

        let result = host.take_pending_view();
        assert!(result.is_none());
    }

    #[test]
    fn test_view_handle_returns_none_after_take() {
        let host = ExtensionHostImpl::new();
        let view = create_test_view();

        let handle = host.render_view_handle(view);
        let handle_id = handle.id().value();

        // Can find handle before take
        assert!(host.view_handle(handle_id).is_some());

        // Take the pending view
        let _ = host.take_pending_view();

        // Cannot find handle after take
        assert!(host.view_handle(handle_id).is_none());
    }

    #[test]
    fn test_clear_view_handles_empties_both_collections() {
        let host = ExtensionHostImpl::new();

        // Add multiple view handles
        for _ in 0..5 {
            host.render_view_handle(create_test_view());
        }

        assert_eq!(host.view_handles.read().len(), 5);
        assert_eq!(host.view_handle_index.read().len(), 5);

        // Clear all handles
        host.clear_view_handles();

        // Both collections should be empty
        assert!(host.view_handles.read().is_empty());
        assert!(host.view_handle_index.read().is_empty());
    }

    #[test]
    fn test_multiple_render_and_take_operations() {
        let host = ExtensionHostImpl::new();

        // Render 3 views
        let handles: Vec<_> = (0..3)
            .map(|_| host.render_view_handle(create_test_view()))
            .collect();

        assert_eq!(host.view_handles.read().len(), 3);
        assert_eq!(host.view_handle_index.read().len(), 3);

        // Take views one by one (LIFO order)
        for i in (0..3).rev() {
            let result = host.take_pending_view();
            assert!(result.is_some());

            // The taken handle should no longer be in the index
            assert!(host.view_handle(handles[i].id().value()).is_none());

            // Remaining counts
            assert_eq!(host.view_handles.read().len(), i);
            assert_eq!(host.view_handle_index.read().len(), i);
        }

        // Final state: both empty
        assert!(host.view_handles.read().is_empty());
        assert!(host.view_handle_index.read().is_empty());
    }
}
