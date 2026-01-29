//! Native extension system for PhotonCast.

pub mod api_bridge;
pub mod cache;
pub mod config;
pub mod context;
pub mod discovery;
pub mod dylib_cache;
pub mod host;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod sandbox;
pub mod signing;
pub mod storage;
pub mod watcher;

pub use api_bridge::{HostViewHandle, HostViewHandleId};
pub use cache::ExtensionCache;
pub use config::{ExtensionConfig, ExtensionPermissionAcceptance, ENV_DEV_EXTENSIONS};
pub use context::make_extension_context;
pub use discovery::{DiscoveryOptions, ExtensionDiscovery};
pub use dylib_cache::{DylibCache, DylibCacheError};
pub use host::{ExtensionHostImpl, ExtensionHostServices};
pub use loader::{ExtensionLibrary, ExtensionLoadError, ExtensionLoader};
pub use manager::{ExtensionManager, ExtensionManagerError, ReloadResult};
pub use manifest::{ExtensionManifest, ManifestError};
pub use permissions::{
    extract_permission_items, permissions_changed, requires_consent, AcceptedPermissions,
    PermissionItem, PermissionType, PermissionsDialog, PermissionsError, PermissionsStore,
};
pub use registry::{ExtensionRegistry, ExtensionState, RegistryError};
pub use runtime::{ExtensionRuntimeImpl, ExtensionRuntimeSpawner};
pub use sandbox::{SandboxError, SandboxedExtension};
pub use storage::{ExtensionStorageImpl, PreferenceStoreImpl};
pub use watcher::{ExtensionWatcher, WatcherError, WatcherEvent};
