use abi_stable::std_types::RString;
use photoncast_extension_api::{ExtensionContext, ExtensionHost};

use crate::extensions::cache::ExtensionCache;
use crate::extensions::host::{ExtensionHostImpl, ExtensionHostServices};
use crate::extensions::manifest::ExtensionManifest;
use crate::extensions::runtime::ExtensionRuntimeImpl;
use crate::extensions::storage::ExtensionStorageImpl;
use crate::utils::paths;

pub fn make_extension_context(
    host_services: &ExtensionHostServices,
    host: &ExtensionHostImpl,
    runtime: &ExtensionRuntimeImpl,
    cache: &ExtensionCache,
    manifest: &ExtensionManifest,
) -> ExtensionContext {
    let extension_data_dir = crate::utils::paths::data_dir()
        .join("extensions")
        .join(&manifest.extension.id);
    let extension_cache_dir = crate::utils::paths::cache_dir()
        .join("extensions")
        .join(&manifest.extension.id);
    let extension_assets_dir = extension_data_dir.join("assets");
    let _ = std::fs::create_dir_all(&extension_data_dir);
    let _ = std::fs::create_dir_all(&extension_cache_dir);
    let _ = std::fs::create_dir_all(&extension_assets_dir);

    ExtensionContext {
        data_dir: RString::from(extension_data_dir.to_string_lossy().as_ref()),
        cache_dir: RString::from(extension_cache_dir.to_string_lossy().as_ref()),
        preferences: host_services.preference_store.api_handle(),
        storage: host_services
            .storage
            .lock()
            .map(|storage| storage.api_handle())
            .unwrap_or_else(|_| {
                ExtensionStorageImpl::new(
                    paths::data_dir().join("extensions_storage.db"),
                    manifest.extension.id.clone(),
                )
                .expect("failed to init extension storage")
                .api_handle()
            }),
        host: ExtensionHost::new(host.clone()),
        runtime: runtime.api_handle(),
        cache: cache.api_handle(),
        extension_id: RString::from(manifest.extension.id.clone()),
        app_version: RString::from(env!("CARGO_PKG_VERSION")),
        assets_dir: RString::from(extension_assets_dir.to_string_lossy().as_ref()),
    }
}
