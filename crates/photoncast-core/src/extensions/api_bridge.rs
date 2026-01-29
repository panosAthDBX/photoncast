use std::sync::atomic::{AtomicU64, Ordering};

use abi_stable::std_types::{ROption, RString, RVec};
use parking_lot::RwLock;
use photoncast_extension_api::{
    ExtensionApiError, ExtensionApiResult, ExtensionView, Toast, ViewHandle, ViewHandleTrait,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostViewHandleId(u64);

impl HostViewHandleId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

static VIEW_HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct HostViewHandle {
    id: HostViewHandleId,
    inner: std::sync::Arc<RwLock<HostViewHandleState>>,
}

#[derive(Debug, Clone)]
struct HostViewHandleState {
    view: Option<ExtensionView>,
    loading: bool,
    error: Option<String>,
}

impl HostViewHandle {
    #[must_use]
    pub fn new() -> Self {
        let id = VIEW_HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            id: HostViewHandleId::new(id),
            inner: std::sync::Arc::new(RwLock::new(HostViewHandleState {
                view: None,
                loading: false,
                error: None,
            })),
        }
    }

    #[must_use]
    pub fn id(&self) -> HostViewHandleId {
        self.id
    }

    pub fn view(&self) -> Option<ExtensionView> {
        self.inner.read().view.clone()
    }

    pub fn loading(&self) -> bool {
        self.inner.read().loading
    }

    pub fn error(&self) -> Option<String> {
        self.inner.read().error.clone()
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn update(&self, view: ExtensionView) {
        let mut state = self.inner.write();
        state.view = Some(view);
        state.error = None;
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn update_items(&self, items: RVec<photoncast_extension_api::ListItem>) {
        let mut state = self.inner.write();
        if let Some(ExtensionView::List(mut list_view)) = state.view.clone() {
            for section in &mut list_view.sections {
                section.items = items.clone();
            }
            state.view = Some(ExtensionView::List(list_view));
        }
    }

    pub fn set_loading(&self, loading: bool) {
        self.inner.write().loading = loading;
    }

    pub fn set_error(&self, error: Option<String>) {
        self.inner.write().error = error;
    }
}

impl Default for HostViewHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct HostViewHandleApi {
    handle: HostViewHandle,
}

impl HostViewHandleApi {
    #[must_use]
    pub fn new(handle: HostViewHandle) -> Self {
        Self { handle }
    }

    #[must_use]
    pub fn into_view_handle(self) -> ViewHandle {
        ViewHandle::new(self)
    }
}

impl ViewHandleTrait for HostViewHandleApi {
    fn update(&self, view: ExtensionView) {
        self.handle.update(view);
    }

    fn update_items(&self, items: RVec<photoncast_extension_api::ListItem>) {
        self.handle.update_items(items);
    }

    fn set_loading(&self, loading: bool) {
        self.handle.set_loading(loading);
    }

    fn set_error(&self, error: ROption<RString>) {
        let error = error
            .into_option()
            .map(photoncast_extension_api::RString::into_string);
        self.handle.set_error(error);
    }
}

pub fn unsupported_result(message: impl Into<String>) -> ExtensionApiResult<()> {
    Err(ExtensionApiError::message(message.into())).into()
}

pub fn toast_from_message(message: impl Into<String>) -> Toast {
    Toast {
        style: photoncast_extension_api::ToastStyle::Default,
        title: RString::from(message.into()),
        message: ROption::RNone,
    }
}
