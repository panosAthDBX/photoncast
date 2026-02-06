//! Extension manager — discovers, loads, and manages extension lifecycles.
//!
//! The [`ExtensionManager`] is the central coordinator for all extension
//! operations: discovery, loading, activation, search integration, command
//! invocation, hot-reload, and permission management.
//!
//! # Security
//!
//! **Current limitation:** Extensions run in-process with full host privileges.
//! A malicious or buggy extension has access to the same memory space and OS
//! capabilities as the host application.
//!
//! **Mitigations in place:**
//! - **Permissions system:** Extensions declare required permissions in their
//!   manifest, and the user must grant consent before activation (see
//!   [`PermissionsStore`] and [`PermissionsDialog`]).
//! - **Code signature verification:** In non-dev mode, every extension dylib
//!   is verified against a code signature before loading.
//! - **Path traversal prevention:** Entry paths are canonicalized and validated
//!   to prevent escaping the extension's base directory.
//!
//! **Future direction:** Consider process-based or WASM-based sandboxing for
//! untrusted extensions to provide stronger isolation guarantees.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;

use abi_stable::std_types::{ROption, RString};
use parking_lot::RwLock;
use thiserror::Error;

use photoncast_extension_api::{
    CommandArguments, CommandInvocationResult, ExtensionApiError, ExtensionApiResult, ExtensionBox,
};

use crate::extensions::cache::ExtensionCache;
use crate::extensions::config::{ExtensionConfig, ExtensionExecutionMode};
use crate::extensions::context::make_extension_context;
use crate::extensions::discovery::{DiscoveryOptions, ExtensionDiscovery};
use crate::extensions::dylib_cache::DylibCache;
use crate::extensions::host::{ExtensionHostImpl, ExtensionHostServices};
use crate::extensions::loader::{ExtensionLibrary, ExtensionLoadError, ExtensionLoader};
use crate::extensions::manifest::{load_manifest, ExtensionManifest, ManifestError, Permissions};
use crate::extensions::permissions::{requires_consent, PermissionsDialog, PermissionsStore};
use crate::extensions::registry::{ExtensionRegistry, ExtensionState, RegistryError};
use crate::extensions::runtime::ExtensionRuntimeImpl;
use crate::extensions::sandbox::{spawn_sandboxed_extension, SandboxError, SandboxedExtension};
use crate::extensions::storage::{ExtensionStorageImpl, PreferenceStoreImpl};
use crate::search::{IconSource, ResultType, SearchAction, SearchResult, SearchResultId};
use crate::utils::paths;

use photoncast_extension_ipc::messages::{
    CommandArguments as IpcCommandArguments, CommandRequest, CommandResponse, SearchRequest,
    SearchResponse,
};
use photoncast_extension_ipc::methods::{EXTENSION_COMMAND, EXTENSION_SEARCH, EXTENSION_SHUTDOWN};

const SANDBOX_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum ExtensionManagerError {
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("load error: {0}")]
    Load(#[from] ExtensionLoadError),
    #[error("registry error: {0}")]
    Registry(#[from] RegistryError),
    #[error("api error: {0}")]
    Api(#[from] ExtensionApiError),
    #[error("extension not enabled: {id}")]
    NotEnabled { id: String },
    #[error("extension not found: {id}")]
    NotFound { id: String },
    #[error("dylib cache error: {0}")]
    DylibCache(#[from] crate::extensions::dylib_cache::DylibCacheError),
    #[error("reload failed: {reason}")]
    ReloadFailed { id: String, reason: String },
    #[error("permissions consent required for extension: {id}")]
    PermissionsConsentRequired {
        id: String,
        dialog: PermissionsDialog,
    },
    #[error("permissions error: {0}")]
    Permissions(#[from] crate::extensions::permissions::PermissionsError),
    #[error("path traversal detected: resolved path {resolved} escapes base directory {base}")]
    PathTraversal { resolved: String, base: String },
    #[error("failed to resolve extension path: {reason}")]
    PathResolutionFailed { reason: String },
    #[error("code signature error: {0}")]
    CodeSignature(#[from] crate::extensions::signing::CodeSignatureError),
    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxError),
}

/// Result of an extension reload operation.
#[derive(Debug)]
pub struct ReloadResult {
    /// The extension ID that was reloaded.
    pub extension_id: String,
    /// Duration of the reload operation.
    pub duration: Duration,
    /// Whether the reload was successful.
    pub success: bool,
    /// Error message if reload failed.
    pub error: Option<String>,
}

impl ReloadResult {
    fn success(extension_id: String, duration: Duration) -> Self {
        Self {
            extension_id,
            duration,
            success: true,
            error: None,
        }
    }

    fn failure(extension_id: String, duration: Duration, error: String) -> Self {
        Self {
            extension_id,
            duration,
            success: false,
            error: Some(error),
        }
    }
}

/// Target reload time in milliseconds.
const RELOAD_TARGET_MS: u64 = 250;

#[derive(Default, Clone)]
pub struct CommandInvocationGuard {
    active: Arc<RwLock<HashSet<String>>>,
}

impl CommandInvocationGuard {
    pub fn is_invocation_allowed(&self, extension_id: &str, command_id: &str) -> bool {
        let key = format!("{extension_id}:{command_id}");
        let mut guard = self.active.write();
        if guard.contains(&key) {
            return false;
        }
        guard.insert(key);
        true
    }

    pub fn complete(&self, extension_id: &str, command_id: &str) {
        let key = format!("{extension_id}:{command_id}");
        self.active.write().remove(&key);
    }
}

pub struct ExtensionManager {
    registry: ExtensionRegistry,
    discovery: ExtensionDiscovery,
    loaded: HashMap<String, LoadedExtension>,
    failure_backoff: HashMap<String, Instant>,
    invocation_guard: CommandInvocationGuard,
    dylib_cache: DylibCache,
    permissions_store: PermissionsStore,
    dev_mode: bool,
    execution_mode: ExtensionExecutionMode,
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self {
            registry: ExtensionRegistry::default(),
            discovery: ExtensionDiscovery::new(),
            loaded: HashMap::new(),
            failure_backoff: HashMap::new(),
            invocation_guard: CommandInvocationGuard::default(),
            dylib_cache: DylibCache::new(paths::cache_dir().join("extensions_dylib")),
            permissions_store: PermissionsStore::load().unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to load extension permissions, using defaults: {}",
                    e
                );
                PermissionsStore::default()
            }),
            dev_mode: false,
            execution_mode: ExtensionExecutionMode::Auto,
        }
    }
}

struct LoadedExtension {
    manifest: ExtensionManifest,
    host: ExtensionHostImpl,
    host_services: ExtensionHostServices,
    kind: LoadedExtensionKind,
}

enum LoadedExtensionKind {
    InProcess(InProcessExtension),
    Sandbox(SandboxedExtension),
}

struct InProcessExtension {
    instance: ExtensionBox,
    runtime: ExtensionRuntimeImpl,
    cache: ExtensionCache,
    /// Keeps the dylib loaded for the extension's lifetime.
    #[allow(dead_code)]
    library: ExtensionLibrary,
    /// Path to the cached dylib (for hot-reload cleanup).
    #[allow(dead_code)]
    cached_dylib_path: Option<PathBuf>,
}

impl ExtensionManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new extension manager with dev mode enabled.
    #[must_use]
    pub fn with_dev_mode(mut self, dev_mode: bool) -> Self {
        self.dev_mode = dev_mode;
        self
    }

    /// Sets the dev mode flag.
    pub fn set_dev_mode(&mut self, dev_mode: bool) {
        self.dev_mode = dev_mode;
    }

    /// Returns whether dev mode is enabled.
    #[must_use]
    pub fn is_dev_mode(&self) -> bool {
        self.dev_mode
    }

    #[must_use]
    pub fn execution_mode(&self) -> ExtensionExecutionMode {
        self.execution_mode
    }

    #[must_use]
    fn effective_execution_mode(&self) -> ExtensionExecutionMode {
        match self.execution_mode {
            ExtensionExecutionMode::Auto => {
                if self.dev_mode {
                    ExtensionExecutionMode::InProcess
                } else {
                    ExtensionExecutionMode::Sandbox
                }
            },
            mode => mode,
        }
    }

    pub fn discover(&mut self, config: &ExtensionConfig) {
        self.dev_mode = config.effective_dev_mode();
        self.execution_mode = config.effective_execution_mode();

        let options = DiscoveryOptions {
            dev_mode: self.dev_mode,
            dev_paths: config.dev_paths.iter().map(PathBuf::from).collect(),
        };

        for result in self.discovery.discover(&options) {
            match result {
                Ok(manifest) => {
                    let enabled = config.enabled;
                    self.registry.insert(manifest, enabled);
                },
                Err(err) => {
                    tracing::warn!(error = %err, "Failed to load extension manifest");
                },
            }
        }
    }

    /// Auto-loads all enabled extensions that don't require permissions consent.
    ///
    /// This should be called after `discover()` to activate extensions for search.
    /// Extensions requiring permissions consent will be skipped and can be loaded
    /// later when the user grants consent.
    #[allow(clippy::result_large_err)]
    pub fn auto_load_enabled(&mut self) {
        let extension_ids: Vec<String> = self
            .registry
            .list()
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.manifest.extension.id.clone())
            .collect();

        for id in extension_ids {
            // Skip if already loaded
            if self.is_loaded(&id) {
                continue;
            }

            match self.load_and_activate(&id) {
                Ok(()) => {
                    tracing::info!(extension_id = %id, "Auto-loaded extension");
                },
                Err(ExtensionManagerError::PermissionsConsentRequired { id, .. }) => {
                    tracing::debug!(
                        extension_id = %id,
                        "Extension requires permissions consent, skipping auto-load"
                    );
                },
                Err(err) => {
                    tracing::warn!(
                        extension_id = %id,
                        error = %err,
                        "Failed to auto-load extension"
                    );
                },
            }
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn load_and_activate(&mut self, id: &str) -> Result<(), ExtensionManagerError> {
        let record = self
            .registry
            .get(id)
            .ok_or_else(|| RegistryError::NotFound { id: id.to_string() })?
            .clone();

        if !record.enabled {
            return Err(ExtensionManagerError::NotEnabled { id: id.to_string() });
        }

        // Check if permissions consent is required
        if requires_consent(&record.manifest.permissions)
            && !self
                .permissions_store
                .has_valid_consent(id, &record.manifest.permissions)
        {
            let dialog = PermissionsDialog::new(
                id,
                &record.manifest.extension.name,
                None, // Extension icon could be added here
                &record.manifest.permissions,
            );
            return Err(ExtensionManagerError::PermissionsConsentRequired {
                id: id.to_string(),
                dialog,
            });
        }

        if let Some(last_failure) = self.failure_backoff.get(id) {
            if last_failure.elapsed() < Duration::from_secs(5) {
                return Err(ExtensionManagerError::Api(ExtensionApiError::message(
                    "extension is rate limited",
                )));
            }
        }

        self.registry.update_state(id, ExtensionState::Loaded)?;

        let entry_path = resolve_entry_path(&record.manifest, None)?;

        // TODO: Consider process-based sandboxing for untrusted extensions.
        // Currently, loaded dylibs run in-process with full host privileges.

        // Verify code signature before loading
        if self.dev_mode {
            tracing::warn!(
                extension_id = id,
                path = %entry_path.display(),
                "Skipping code signature verification in dev mode"
            );
        } else {
            super::signing::verify_code_signature(&entry_path)?;
        }

        let host_services = ExtensionHostServices {
            preference_store: PreferenceStoreImpl::new(record.manifest.preferences.clone()),
            storage: std::sync::Arc::new(std::sync::Mutex::new(
                ExtensionStorageImpl::new(
                    paths::data_dir().join("extensions_storage.db"),
                    record.manifest.extension.id.clone(),
                )
                .map_err(|e| ExtensionApiError::message(e.to_string()))?,
            )),
            command_invocation_guard: self.invocation_guard.clone(),
            allowed_filesystem_paths: record
                .manifest
                .permissions
                .filesystem
                .iter()
                .map(std::path::PathBuf::from)
                .collect(),
        };
        let host = ExtensionHostImpl::with_services(host_services.clone());
        match self.effective_execution_mode() {
            ExtensionExecutionMode::Sandbox => {
                let sandbox =
                    spawn_sandboxed_extension(&record.manifest, &entry_path, host.clone())?;
                let loaded = LoadedExtension {
                    manifest: record.manifest.clone(),
                    host,
                    host_services,
                    kind: LoadedExtensionKind::Sandbox(sandbox),
                };
                self.registry.update_state(id, ExtensionState::Active)?;
                self.loaded.insert(id.to_string(), loaded);
                Ok(())
            },
            ExtensionExecutionMode::InProcess => {
                let library = ExtensionLoader::load(&entry_path)?;
                let api_version = ExtensionLoader::resolve_api_version(library.raw())?;
                ExtensionLoader::check_api_version(api_version)?;

                let runtime = ExtensionRuntimeImpl::new();
                let cache = ExtensionCache::new(
                    record.manifest.extension.id.clone(),
                    paths::cache_dir()
                        .join("extensions")
                        .join(&record.manifest.extension.id),
                );

                // Note: directory creation for extension data/cache/assets is handled
                // by `make_extension_context()` in context.rs, so we don't duplicate it here.

                let root_module = ExtensionLoader::load_root_module(&library)?;
                let instance = root_module.instantiate_extension();

                let mut in_process = InProcessExtension {
                    instance,
                    runtime,
                    cache,
                    library,
                    cached_dylib_path: None,
                };

                if let Err(err) = in_process
                    .instance
                    .activate(make_extension_context(
                        &host_services,
                        &host,
                        &in_process.runtime,
                        &in_process.cache,
                        &record.manifest,
                    ))
                    .into_result()
                {
                    return Err(ExtensionManagerError::Api(err));
                }

                let startup_ctx = make_extension_context(
                    &host_services,
                    &host,
                    &in_process.runtime,
                    &in_process.cache,
                    &record.manifest,
                );
                if let Err(err) = in_process.instance.on_startup(&startup_ctx).into_result() {
                    tracing::warn!(
                        extension_id = id,
                        error = %err,
                        "Extension on_startup hook failed"
                    );
                }

                let loaded = LoadedExtension {
                    manifest: record.manifest.clone(),
                    host,
                    host_services,
                    kind: LoadedExtensionKind::InProcess(in_process),
                };

                self.registry.update_state(id, ExtensionState::Active)?;
                self.loaded.insert(id.to_string(), loaded);
                Ok(())
            },
            ExtensionExecutionMode::Auto => Err(ExtensionManagerError::Sandbox(
                SandboxError::MissingManifestDir {
                    extension_id: record.manifest.extension.id.clone(),
                },
            )),
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn deactivate_and_unload(&mut self, id: &str) -> Result<(), ExtensionManagerError> {
        if let Some(loaded) = self.loaded.remove(id) {
            match loaded.kind {
                LoadedExtensionKind::InProcess(mut in_process) => {
                    if let Err(err) = in_process.instance.deactivate().into_result() {
                        return Err(ExtensionManagerError::Api(err));
                    }
                },
                LoadedExtensionKind::Sandbox(mut sandbox) => {
                    if let Err(err) = sandbox
                        .connection
                        .send_request(EXTENSION_SHUTDOWN, Value::Null)
                    {
                        tracing::warn!(extension_id = %id, error = %err, "Failed to send sandbox shutdown request");
                    }
                    if let Err(err) = wait_for_sandbox_exit(&mut sandbox.process) {
                        tracing::warn!(extension_id = %id, error = %err, "Failed to stop sandbox process");
                    }
                },
            }
            self.registry.update_state(id, ExtensionState::Unloaded)?;
        }
        Ok(())
    }

    // ========================================================================
    // Permission Consent Management
    // ========================================================================

    /// Checks if an extension requires permissions consent before activation.
    ///
    /// Returns `Some(PermissionsDialog)` if consent is needed, `None` otherwise.
    #[must_use]
    #[allow(clippy::result_large_err)]
    pub fn check_permissions_consent(&self, id: &str) -> Option<PermissionsDialog> {
        let record = self.registry.get(id)?;
        let permissions = &record.manifest.permissions;

        if !requires_consent(permissions) {
            return None;
        }

        if self.permissions_store.has_valid_consent(id, permissions) {
            return None;
        }

        Some(PermissionsDialog::new(
            id,
            &record.manifest.extension.name,
            None,
            permissions,
        ))
    }

    /// Accepts permissions for an extension, allowing it to be activated.
    ///
    /// Call this method after the user approves the permissions dialog.
    #[allow(clippy::result_large_err)]
    pub fn accept_permissions(&mut self, id: &str) -> Result<(), ExtensionManagerError> {
        let record = self
            .registry
            .get(id)
            .ok_or_else(|| RegistryError::NotFound { id: id.to_string() })?;

        self.permissions_store
            .accept_permissions(id, &record.manifest.permissions);

        // Persist to disk
        self.permissions_store.save()?;

        tracing::info!(extension_id = id, "Permissions accepted for extension");
        Ok(())
    }

    /// Revokes permissions for an extension.
    ///
    /// The extension will need to request consent again on next activation.
    #[allow(clippy::result_large_err)]
    pub fn revoke_permissions(&mut self, id: &str) -> Result<(), ExtensionManagerError> {
        self.permissions_store.revoke_permissions(id);
        self.permissions_store.save()?;

        tracing::info!(extension_id = id, "Permissions revoked for extension");
        Ok(())
    }

    /// Returns the permissions requested by an extension.
    #[must_use]
    pub fn get_extension_permissions(&self, id: &str) -> Option<&Permissions> {
        self.registry.get(id).map(|r| &r.manifest.permissions)
    }

    /// Returns whether an extension has valid permissions consent.
    #[must_use]
    pub fn has_permissions_consent(&self, id: &str) -> bool {
        let Some(record) = self.registry.get(id) else {
            return false;
        };

        !requires_consent(&record.manifest.permissions)
            || self
                .permissions_store
                .has_valid_consent(id, &record.manifest.permissions)
    }

    // ========================================================================
    // Extension Hot-Reload
    // ========================================================================

    /// Reloads an extension in dev mode.
    ///
    /// This performs the full reload pipeline:
    /// 1. Deactivate the extension
    /// 2. Unload the library
    /// 3. Create a versioned copy of the dylib (to bypass OS caching)
    /// 4. Load the new library
    /// 5. Activate the extension
    ///
    /// # Arguments
    ///
    /// * `id` - The extension ID to reload.
    /// * `extension_path` - The path to the extension directory (for re-reading manifest).
    ///
    /// # Returns
    ///
    /// A `ReloadResult` containing timing information and success status.
    pub fn reload_extension(&mut self, id: &str, extension_path: &Path) -> ReloadResult {
        if self.effective_execution_mode() == ExtensionExecutionMode::Sandbox {
            return ReloadResult::failure(
                id.to_string(),
                Duration::ZERO,
                "Reload is not supported for sandboxed extensions".to_string(),
            );
        }

        let start = Instant::now();

        tracing::info!(
            extension_id = id,
            path = %extension_path.display(),
            "Starting extension reload"
        );

        // Step 1: Deactivate and unload current instance
        if let Err(err) = self.deactivate_and_unload(id) {
            let duration = start.elapsed();
            let error_msg = format!("Failed to deactivate: {err}");
            tracing::error!(
                extension_id = id,
                duration_ms = duration.as_millis(),
                error = %err,
                "Extension reload failed during deactivation"
            );
            self.mark_failed(id, &error_msg);
            return ReloadResult::failure(id.to_string(), duration, error_msg);
        }

        // Step 2: Re-read the manifest (in case it changed)
        let manifest_path = extension_path.join("extension.toml");
        let manifest = match load_manifest(&manifest_path) {
            Ok(m) => m,
            Err(err) => {
                let duration = start.elapsed();
                let error_msg = format!("Failed to read manifest: {err}");
                tracing::error!(
                    extension_id = id,
                    duration_ms = duration.as_millis(),
                    error = %err,
                    "Extension reload failed reading manifest"
                );
                self.mark_failed(id, &error_msg);
                return ReloadResult::failure(id.to_string(), duration, error_msg);
            },
        };

        // Update registry with new manifest
        self.registry.insert(manifest.clone(), true);

        // Step 3: Create versioned copy of dylib for hot-reload
        let entry_path = match resolve_entry_path(&manifest, None) {
            Ok(p) => p,
            Err(err) => {
                let duration = start.elapsed();
                let error_msg = format!("Path resolution failed: {err}");
                tracing::error!(
                    extension_id = id,
                    duration_ms = duration.as_millis(),
                    error = %err,
                    "Extension reload failed resolving entry path"
                );
                self.mark_failed(id, &error_msg);
                return ReloadResult::failure(id.to_string(), duration, error_msg);
            },
        };

        // Verify code signature before loading
        if self.dev_mode {
            tracing::warn!(
                extension_id = id,
                path = %entry_path.display(),
                "Skipping code signature verification in dev mode"
            );
        } else if let Err(err) = super::signing::verify_code_signature(&entry_path) {
            let duration = start.elapsed();
            let error_msg = format!("Code signature verification failed: {err}");
            tracing::error!(
                extension_id = id,
                duration_ms = duration.as_millis(),
                error = %err,
                "Extension reload failed code signature verification"
            );
            self.mark_failed(id, &error_msg);
            return ReloadResult::failure(id.to_string(), duration, error_msg);
        }

        let load_path = if self.dev_mode {
            match self.dylib_cache.create_versioned_copy(id, &entry_path) {
                Ok(cached_path) => {
                    tracing::debug!(
                        extension_id = id,
                        cached = %cached_path.display(),
                        "Using versioned dylib copy"
                    );
                    cached_path
                },
                Err(err) => {
                    tracing::warn!(
                        extension_id = id,
                        error = %err,
                        "Failed to create versioned dylib copy, using original"
                    );
                    entry_path.clone()
                },
            }
        } else {
            entry_path.clone()
        };

        // Step 4: Load the new library
        let library = match ExtensionLoader::load(&load_path) {
            Ok(lib) => lib,
            Err(err) => {
                let duration = start.elapsed();
                let error_msg = format!("Failed to load library: {err}");
                tracing::error!(
                    extension_id = id,
                    duration_ms = duration.as_millis(),
                    error = %err,
                    "Extension reload failed loading library"
                );
                self.mark_failed(id, &error_msg);
                return ReloadResult::failure(id.to_string(), duration, error_msg);
            },
        };

        // Verify API version
        let api_version = match ExtensionLoader::resolve_api_version(library.raw()) {
            Ok(v) => v,
            Err(err) => {
                let duration = start.elapsed();
                let error_msg = format!("Failed to resolve API version: {err}");
                self.mark_failed(id, &error_msg);
                return ReloadResult::failure(id.to_string(), duration, error_msg);
            },
        };
        if let Err(err) = ExtensionLoader::check_api_version(api_version) {
            let duration = start.elapsed();
            let error_msg = format!("API version mismatch: {err}");
            self.mark_failed(id, &error_msg);
            return ReloadResult::failure(id.to_string(), duration, error_msg);
        }

        // Transition state to Loaded
        if let Err(err) = self.registry.update_state(id, ExtensionState::Loaded) {
            let duration = start.elapsed();
            let error_msg = format!("Failed to update state: {err}");
            self.mark_failed(id, &error_msg);
            return ReloadResult::failure(id.to_string(), duration, error_msg);
        }

        // Step 5: Create host services and activate
        let host_services = ExtensionHostServices {
            preference_store: PreferenceStoreImpl::new(manifest.preferences.clone()),
            storage: std::sync::Arc::new(std::sync::Mutex::new(
                match ExtensionStorageImpl::new(
                    paths::data_dir().join("extensions_storage.db"),
                    manifest.extension.id.clone(),
                ) {
                    Ok(s) => s,
                    Err(err) => {
                        let duration = start.elapsed();
                        let error_msg = format!("Failed to create storage: {err}");
                        self.mark_failed(id, &error_msg);
                        return ReloadResult::failure(id.to_string(), duration, error_msg);
                    },
                },
            )),
            command_invocation_guard: self.invocation_guard.clone(),
            allowed_filesystem_paths: manifest
                .permissions
                .filesystem
                .iter()
                .map(std::path::PathBuf::from)
                .collect(),
        };
        let host = ExtensionHostImpl::with_services(host_services.clone());
        let runtime = ExtensionRuntimeImpl::new();
        let cache = ExtensionCache::new(
            manifest.extension.id.clone(),
            paths::cache_dir()
                .join("extensions")
                .join(&manifest.extension.id),
        );

        let root_module = match ExtensionLoader::load_root_module(&library) {
            Ok(m) => m,
            Err(err) => {
                let duration = start.elapsed();
                let error_msg = format!("Failed to load root module: {err}");
                self.mark_failed(id, &error_msg);
                return ReloadResult::failure(id.to_string(), duration, error_msg);
            },
        };

        let instance = root_module.instantiate_extension();

        let cached_dylib_path = if load_path == entry_path {
            None
        } else {
            Some(load_path)
        };

        let mut in_process = InProcessExtension {
            instance,
            runtime,
            cache,
            library,
            cached_dylib_path: cached_dylib_path.clone(),
        };

        // Activate the extension
        if let Err(err) = in_process
            .instance
            .activate(make_extension_context(
                &host_services,
                &host,
                &in_process.runtime,
                &in_process.cache,
                &manifest,
            ))
            .into_result()
        {
            let duration = start.elapsed();
            let error_msg = format!("Failed to activate: {err}");
            tracing::error!(
                extension_id = id,
                duration_ms = duration.as_millis(),
                error = %err,
                "Extension reload failed during activation"
            );
            self.mark_failed(id, &error_msg);
            return ReloadResult::failure(id.to_string(), duration, error_msg);
        }

        // Call on_startup hook (errors are logged but don't prevent activation)
        let startup_ctx = make_extension_context(
            &host_services,
            &host,
            &in_process.runtime,
            &in_process.cache,
            &manifest,
        );
        if let Err(err) = in_process.instance.on_startup(&startup_ctx).into_result() {
            tracing::warn!(
                extension_id = id,
                error = %err,
                "Extension on_startup hook failed during reload"
            );
        }

        // Update state to Active
        if let Err(err) = self.registry.update_state(id, ExtensionState::Active) {
            let duration = start.elapsed();
            let error_msg = format!("Failed to update state to Active: {err}");
            self.mark_failed(id, &error_msg);
            return ReloadResult::failure(id.to_string(), duration, error_msg);
        }

        let loaded = LoadedExtension {
            manifest: manifest.clone(),
            host,
            host_services,
            kind: LoadedExtensionKind::InProcess(in_process),
        };

        self.loaded.insert(id.to_string(), loaded);

        // Clear failure backoff on success
        self.failure_backoff.remove(id);
        self.registry.set_error(id, None);

        // Cleanup old dylib versions
        if self.dev_mode {
            if let Err(err) = self
                .dylib_cache
                .cleanup_old_versions(id, cached_dylib_path.as_deref())
            {
                tracing::warn!(
                    extension_id = id,
                    error = %err,
                    "Failed to cleanup old dylib versions"
                );
            }
        }

        let duration = start.elapsed();
        let duration_ms = duration.as_millis();

        if duration_ms > u128::from(RELOAD_TARGET_MS) {
            tracing::warn!(
                extension_id = id,
                duration_ms = duration_ms,
                target_ms = RELOAD_TARGET_MS,
                "Extension reload exceeded target time"
            );
        } else {
            tracing::info!(
                extension_id = id,
                duration_ms = duration_ms,
                "Extension reloaded successfully"
            );
        }

        ReloadResult::success(id.to_string(), duration)
    }

    /// Returns the path to an extension's root directory if it's currently loaded.
    #[must_use]
    pub fn get_extension_path(&self, id: &str) -> Option<PathBuf> {
        self.registry.get(id).map(|record| {
            PathBuf::from(&record.manifest.entry.path)
                .parent()
                .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf)
        })
    }

    /// Returns whether an extension is currently loaded.
    #[must_use]
    pub fn is_loaded(&self, id: &str) -> bool {
        self.loaded.contains_key(id)
    }

    pub fn mark_failed(&mut self, id: &str, error: impl Into<String>) {
        self.registry.set_error(id, Some(error.into()));
        let _ = self.registry.update_state(id, ExtensionState::Failed);
        self.failure_backoff.insert(id.to_string(), Instant::now());
    }

    pub fn registry(&self) -> &ExtensionRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ExtensionRegistry {
        &mut self.registry
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        thread_local! {
            static MATCHER: std::cell::RefCell<crate::search::fuzzy::FuzzyMatcher> =
                std::cell::RefCell::new(crate::search::fuzzy::FuzzyMatcher::default());
            static COMMAND_MATCHER: std::cell::RefCell<crate::search::fuzzy::FuzzyMatcher> =
                std::cell::RefCell::new(crate::search::fuzzy::FuzzyMatcher::default());
        }

        MATCHER.with_borrow_mut(|matcher| {
            COMMAND_MATCHER.with_borrow_mut(|command_matcher| {
                for loaded in self.loaded.values() {
                    let extension_id = loaded.manifest.extension.id.clone();
                    match &loaded.kind {
                        LoadedExtensionKind::InProcess(in_process) => {
                            let provider = in_process.instance.search_provider().into_option();
                            if let Some(provider) = provider {
                                let items = provider.search(RString::from(query), max_results);
                                for item in items {
                                    let title = item.title.into_string();
                                    let match_indices = matcher
                                        .score(query, &title)
                                        .map(|(_, indices)| indices)
                                        .unwrap_or_default();

                                    results.push(SearchResult {
                                        id: SearchResultId::new(format!(
                                            "ext:{}:{}",
                                            extension_id, item.id
                                        )),
                                        title,
                                        subtitle: item
                                            .subtitle
                                            .clone()
                                            .map(photoncast_extension_api::RString::into_string)
                                            .unwrap_or_default(),
                                        icon: map_extension_icon(item.icon),
                                        result_type: ResultType::Extension,
                                        score: item.score,
                                        match_indices,
                                        action: SearchAction::ExecuteExtensionCommand {
                                            extension_id: extension_id.clone(),
                                            command_id: item.id.to_string(),
                                        },
                                        requires_permissions: false,
                                    });
                                }
                            }

                            let commands = in_process.instance.commands();
                            for command in commands {
                                let name = command.name.to_string();
                                let mut best_score =
                                    command_matcher.score(query, &name).map(|(score, indices)| {
                                        (score.saturating_add(100), indices, true)
                                    });

                                if let Some((score, indices)) =
                                    command_matcher.score(query, command.id.as_str())
                                {
                                    match &best_score {
                                        Some((best, _, _)) if score <= *best => {},
                                        _ => {
                                            best_score =
                                                Some((score.saturating_add(50), indices, false));
                                        },
                                    }
                                }

                                for keyword in &command.keywords {
                                    if let Some((score, indices)) =
                                        command_matcher.score(query, keyword.as_str())
                                    {
                                        match &best_score {
                                            Some((best, _, _)) if score <= *best => {},
                                            _ => {
                                                best_score = Some((score, indices, false));
                                            },
                                        }
                                    }
                                }

                                let Some((score, indices, name_match)) = best_score else {
                                    continue;
                                };

                                let subtitle = command.subtitle.clone().into_option().map_or_else(
                                    || loaded.manifest.extension.name.clone(),
                                    photoncast_extension_api::RString::into_string,
                                );

                                #[allow(clippy::map_unwrap_or)]
                                let icon = command
                                    .icon
                                    .clone()
                                    .into_option()
                                    .map(map_extension_icon)
                                    .unwrap_or_else(|| IconSource::SystemIcon {
                                        name: "puzzlepiece".to_string(),
                                    });

                                results.push(SearchResult {
                                    id: SearchResultId::new(format!(
                                        "ext-command:{}:{}",
                                        extension_id, command.id
                                    )),
                                    title: name,
                                    subtitle,
                                    icon,
                                    result_type: ResultType::Extension,
                                    score: f64::from(score),
                                    match_indices: if name_match { indices } else { Vec::new() },
                                    action: SearchAction::ExecuteExtensionCommand {
                                        extension_id: extension_id.clone(),
                                        command_id: command.id.to_string(),
                                    },
                                    requires_permissions: false,
                                });
                            }
                        },
                        LoadedExtensionKind::Sandbox(sandbox) => {
                            let request = SearchRequest {
                                query: query.to_string(),
                                max_results,
                            };
                            let Ok(params) = serde_json::to_value(request) else {
                                continue;
                            };
                            let Ok(response_value) =
                                sandbox.connection.send_request(EXTENSION_SEARCH, params)
                            else {
                                continue;
                            };
                            let Ok(response) =
                                serde_json::from_value::<SearchResponse>(response_value)
                            else {
                                continue;
                            };

                            for item in response.items {
                                let match_indices = matcher
                                    .score(query, &item.title)
                                    .map(|(_, indices)| indices)
                                    .unwrap_or_default();

                                results.push(SearchResult {
                                    id: SearchResultId::new(format!(
                                        "ext:{}:{}",
                                        extension_id, item.id
                                    )),
                                    title: item.title,
                                    subtitle: item.subtitle.unwrap_or_default(),
                                    icon: map_extension_icon(item.icon),
                                    result_type: ResultType::Extension,
                                    score: item.score,
                                    match_indices,
                                    action: SearchAction::ExecuteExtensionCommand {
                                        extension_id: extension_id.clone(),
                                        command_id: item.id.clone(),
                                    },
                                    requires_permissions: false,
                                });
                            }
                        },
                    }
                }

                // Also search commands from unloaded but enabled extensions
                for record in self.registry.list() {
                    // Skip if already loaded or disabled
                    if self.loaded.contains_key(&record.manifest.extension.id) || !record.enabled {
                        continue;
                    }

                    let extension_id = &record.manifest.extension.id;
                    let extension_name = &record.manifest.extension.name;

                    // Check if this extension needs permissions consent
                    let needs_consent = self.check_permissions_consent(extension_id).is_some();

                    for command in &record.manifest.commands {
                        let name = &command.name;
                        let mut best_score = command_matcher
                            .score(query, name)
                            .map(|(score, indices)| (score.saturating_add(100), indices, true));

                        if let Some((score, indices)) = command_matcher.score(query, &command.id) {
                            match &best_score {
                                Some((best, _, _)) if score <= *best => {},
                                _ => {
                                    best_score = Some((score.saturating_add(50), indices, false));
                                },
                            }
                        }

                        for keyword in &command.keywords {
                            if let Some((score, indices)) = command_matcher.score(query, keyword) {
                                match &best_score {
                                    Some((best, _, _)) if score <= *best => {},
                                    _ => {
                                        best_score = Some((score, indices, false));
                                    },
                                }
                            }
                        }

                        let Some((score, indices, name_match)) = best_score else {
                            continue;
                        };

                        let subtitle = command.subtitle.clone().unwrap_or_else(|| {
                            if needs_consent {
                                format!("{extension_name} (requires permissions)")
                            } else {
                                extension_name.clone()
                            }
                        });

                        #[allow(clippy::map_unwrap_or)]
                        let icon = command
                            .icon
                            .clone()
                            .map(|i| IconSource::SystemIcon { name: i })
                            .unwrap_or_else(|| IconSource::SystemIcon {
                                name: "puzzlepiece".to_string(),
                            });

                        results.push(SearchResult {
                            id: SearchResultId::new(format!(
                                "ext-command:{}:{}",
                                extension_id, command.id
                            )),
                            title: name.clone(),
                            subtitle,
                            icon,
                            result_type: ResultType::Extension,
                            score: f64::from(score),
                            match_indices: if name_match { indices } else { Vec::new() },
                            action: SearchAction::ExecuteExtensionCommand {
                                extension_id: extension_id.clone(),
                                command_id: command.id.clone(),
                            },
                            requires_permissions: needs_consent,
                        });
                    }
                }

                // Also include sandboxed extensions' manifest commands to support command-only entries.
                for loaded in self.loaded.values() {
                    if !matches!(loaded.kind, LoadedExtensionKind::Sandbox(_)) {
                        continue;
                    }

                    let extension_id = &loaded.manifest.extension.id;
                    let extension_name = &loaded.manifest.extension.name;
                    let needs_consent = self.check_permissions_consent(extension_id).is_some();

                    for command in &loaded.manifest.commands {
                        let name = &command.name;
                        let mut best_score = command_matcher
                            .score(query, name)
                            .map(|(score, indices)| (score.saturating_add(100), indices, true));

                        if let Some((score, indices)) = command_matcher.score(query, &command.id) {
                            match &best_score {
                                Some((best, _, _)) if score <= *best => {},
                                _ => {
                                    best_score = Some((score.saturating_add(50), indices, false));
                                },
                            }
                        }

                        for keyword in &command.keywords {
                            if let Some((score, indices)) = command_matcher.score(query, keyword) {
                                match &best_score {
                                    Some((best, _, _)) if score <= *best => {},
                                    _ => {
                                        best_score = Some((score, indices, false));
                                    },
                                }
                            }
                        }

                        let Some((score, indices, name_match)) = best_score else {
                            continue;
                        };

                        let subtitle = command.subtitle.clone().unwrap_or_else(|| {
                            if needs_consent {
                                format!("{extension_name} (requires permissions)")
                            } else {
                                extension_name.clone()
                            }
                        });

                        #[allow(clippy::map_unwrap_or)]
                        let icon = command
                            .icon
                            .clone()
                            .map(|i| IconSource::SystemIcon { name: i })
                            .unwrap_or_else(|| IconSource::SystemIcon {
                                name: "puzzlepiece".to_string(),
                            });

                        results.push(SearchResult {
                            id: SearchResultId::new(format!(
                                "ext-command:{}:{}",
                                extension_id, command.id
                            )),
                            title: name.clone(),
                            subtitle,
                            icon,
                            result_type: ResultType::Extension,
                            score: f64::from(score),
                            match_indices: if name_match { indices } else { Vec::new() },
                            action: SearchAction::ExecuteExtensionCommand {
                                extension_id: extension_id.clone(),
                                command_id: command.id.clone(),
                            },
                            requires_permissions: needs_consent,
                        });
                    }
                }
            }); // COMMAND_MATCHER
        }); // MATCHER

        results.sort_by(|a, b| b.score.total_cmp(&a.score));
        results.truncate(max_results);
        results
    }

    #[allow(clippy::manual_let_else)]
    pub fn launch_command(
        &mut self,
        extension_id: &str,
        command_id: &str,
        args: CommandArguments,
    ) -> ExtensionApiResult<CommandInvocationResult> {
        if !self.registry.get(extension_id).is_some_and(|r| r.enabled) {
            return Err(ExtensionApiError::message("extension disabled")).into();
        }

        if !self.loaded.contains_key(extension_id) {
            if let Err(err) = self.load_and_activate(extension_id) {
                return Err(ExtensionApiError::message(format!("{err}"))).into();
            }
        }

        let loaded = match self.loaded.get(extension_id) {
            Some(loaded) => loaded,
            None => return Err(ExtensionApiError::message("extension not loaded")).into(),
        };

        match &loaded.kind {
            LoadedExtensionKind::InProcess(in_process) => {
                for command in in_process.instance.commands() {
                    if command.id.as_str() == command_id {
                        if let Err(err) = command
                            .handler
                            .handle(
                                make_extension_context(
                                    &loaded.host_services,
                                    &loaded.host,
                                    &in_process.runtime,
                                    &in_process.cache,
                                    &loaded.manifest,
                                ),
                                args,
                            )
                            .into_result()
                        {
                            return Err(ExtensionApiError::message(format!("{err}"))).into();
                        }
                        self.invocation_guard.complete(extension_id, command_id);
                        return Ok(CommandInvocationResult {
                            success: true,
                            message: ROption::RNone,
                        })
                        .into();
                    }
                }
            },
            LoadedExtensionKind::Sandbox(sandbox) => {
                let args = match api_args_to_ipc(args).into_result() {
                    Ok(value) => value,
                    Err(err) => return Err(err).into(),
                };
                let request = CommandRequest {
                    command_id: command_id.to_string(),
                    args,
                };
                let params = match serde_json::to_value(request)
                    .map_err(|err| ExtensionApiError::message(err.to_string()))
                {
                    Ok(value) => value,
                    Err(err) => return Err(err).into(),
                };
                let response = match sandbox
                    .connection
                    .send_request(EXTENSION_COMMAND, params)
                    .map_err(|err| ExtensionApiError::message(err.to_string()))
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
                self.invocation_guard.complete(extension_id, command_id);
                return Ok(CommandInvocationResult {
                    success: response.success,
                    message: response.message.map(RString::from).into(),
                })
                .into();
            },
        }

        self.invocation_guard.complete(extension_id, command_id);
        Err(ExtensionApiError::message("command not found")).into()
    }

    /// Gets the most recently rendered view for an extension (and clears it).
    ///
    /// This should be called after `launch_command` to retrieve any view
    /// that the extension rendered during command execution.
    #[must_use]
    pub fn take_pending_view(
        &mut self,
        extension_id: &str,
    ) -> Option<photoncast_extension_api::ExtensionView> {
        let loaded = self.loaded.get(extension_id)?;
        // Use host's method which properly cleans up both view_handles and view_handle_index
        loaded.host.take_pending_view()
    }

    /// Clears all pending view handles for an extension.
    /// Call this when an extension command completes to prevent memory leaks.
    pub fn clear_extension_view_handles(&mut self, extension_id: &str) {
        if let Some(loaded) = self.loaded.get(extension_id) {
            loaded.host.clear_view_handles();
        }
    }
}

fn wait_for_sandbox_exit(child: &mut std::process::Child) -> Result<(), std::io::Error> {
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        if start.elapsed() > SANDBOX_SHUTDOWN_TIMEOUT {
            let _ = child.kill();
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn api_args_to_ipc(args: CommandArguments) -> ExtensionApiResult<IpcCommandArguments> {
    let extra = match args
        .extra
        .into_option()
        .map(|value| serde_json::from_str::<Value>(value.get()))
        .transpose()
    {
        Ok(value) => value,
        Err(err) => return Err(ExtensionApiError::message(err.to_string())).into(),
    };

    Ok(IpcCommandArguments {
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
    })
    .into()
}

fn map_extension_icon(icon: photoncast_extension_api::IconSource) -> IconSource {
    match icon {
        photoncast_extension_api::IconSource::SystemIcon { name } => IconSource::SystemIcon {
            name: name.into_string(),
        },
        photoncast_extension_api::IconSource::Emoji { glyph } => IconSource::Emoji {
            char: glyph.as_str().chars().next().unwrap_or('🔗'),
        },
        photoncast_extension_api::IconSource::AppIcon {
            bundle_id,
            icon_path,
        } => IconSource::AppIcon {
            bundle_id: bundle_id.into_string(),
            icon_path: icon_path
                .into_option()
                .map(|path| PathBuf::from(path.into_string())),
        },
        photoncast_extension_api::IconSource::FileIcon { path } => IconSource::FileIcon {
            path: PathBuf::from(path.into_string()),
        },
    }
}

#[allow(clippy::manual_let_else, clippy::result_large_err)]
fn resolve_entry_path(
    manifest: &ExtensionManifest,
    override_path: Option<&Path>,
) -> Result<PathBuf, ExtensionManagerError> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }

    // Use the manifest's directory (set during discovery) to resolve the entry path
    let base_dir = manifest
        .directory
        .as_deref()
        .unwrap_or_else(|| Path::new("."));

    let joined = base_dir.join(&manifest.entry.path);

    // Canonicalize both paths to resolve symlinks and ".." components.
    // Fail closed: if either path cannot be canonicalized (e.g. doesn't exist),
    // reject rather than returning the unchecked joined path.
    let canonical_base = std::fs::canonicalize(base_dir).map_err(|e| {
        ExtensionManagerError::PathResolutionFailed {
            reason: format!(
                "failed to canonicalize base directory '{}': {e}",
                base_dir.display()
            ),
        }
    })?;
    let canonical_resolved = std::fs::canonicalize(&joined).map_err(|e| {
        ExtensionManagerError::PathResolutionFailed {
            reason: format!(
                "failed to canonicalize entry path '{}': {e}",
                joined.display()
            ),
        }
    })?;

    if !canonical_resolved.starts_with(&canonical_base) {
        return Err(ExtensionManagerError::PathTraversal {
            resolved: canonical_resolved.display().to_string(),
            base: canonical_base.display().to_string(),
        });
    }

    Ok(canonical_resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // ExtensionManager Creation and Defaults
    // =============================================================================

    #[test]
    fn test_manager_creation() {
        let manager = ExtensionManager::new();
        assert!(!manager.is_dev_mode());
        assert!(!manager.is_loaded("nonexistent"));
    }

    #[test]
    fn test_manager_with_dev_mode() {
        let manager = ExtensionManager::new().with_dev_mode(true);
        assert!(manager.is_dev_mode());
    }

    #[test]
    fn test_manager_set_dev_mode() {
        let mut manager = ExtensionManager::new();
        assert!(!manager.is_dev_mode());
        manager.set_dev_mode(true);
        assert!(manager.is_dev_mode());
        manager.set_dev_mode(false);
        assert!(!manager.is_dev_mode());
    }

    // =============================================================================
    // Extension Loading State
    // =============================================================================

    #[test]
    fn test_is_loaded_returns_false_for_unknown_extension() {
        let manager = ExtensionManager::new();
        assert!(!manager.is_loaded("com.example.unknown"));
        assert!(!manager.is_loaded(""));
    }

    #[test]
    fn test_get_extension_path_returns_none_for_unknown() {
        let manager = ExtensionManager::new();
        assert!(manager.get_extension_path("com.example.unknown").is_none());
    }

    #[test]
    fn test_get_extension_permissions_returns_none_for_unknown() {
        let manager = ExtensionManager::new();
        assert!(manager
            .get_extension_permissions("com.example.unknown")
            .is_none());
    }

    // =============================================================================
    // Search Integration
    // =============================================================================

    #[test]
    fn test_search_returns_empty_for_no_extensions() {
        let manager = ExtensionManager::new();
        let results = manager.search("test", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_respects_max_results() {
        let manager = ExtensionManager::new();
        // Even with no extensions, verify max_results parameter works
        let results = manager.search("test", 0);
        assert!(results.is_empty());
    }

    // =============================================================================
    // Command Invocation Guard
    // =============================================================================

    #[test]
    fn test_command_invocation_guard_allows_first_call() {
        let guard = CommandInvocationGuard::default();
        assert!(guard.is_invocation_allowed("ext1", "cmd1"));
    }

    #[test]
    fn test_command_invocation_guard_prevents_circular() {
        let guard = CommandInvocationGuard::default();

        // First call should be allowed
        assert!(guard.is_invocation_allowed("ext1", "cmd1"));

        // Same call again should be blocked (circular)
        assert!(!guard.is_invocation_allowed("ext1", "cmd1"));

        // Different command should be allowed
        assert!(guard.is_invocation_allowed("ext1", "cmd2"));

        // Complete the first call
        guard.complete("ext1", "cmd1");

        // Now the first call should be allowed again
        assert!(guard.is_invocation_allowed("ext1", "cmd1"));
    }

    #[test]
    fn test_command_invocation_guard_different_extensions() {
        let guard = CommandInvocationGuard::default();

        // Start invocation for ext1
        assert!(guard.is_invocation_allowed("ext1", "cmd1"));

        // Different extension should still be allowed
        assert!(guard.is_invocation_allowed("ext2", "cmd1"));

        // Cleanup
        guard.complete("ext1", "cmd1");
        guard.complete("ext2", "cmd1");
    }

    // =============================================================================
    // ReloadResult
    // =============================================================================

    #[test]
    fn test_reload_result_success() {
        let result = ReloadResult::success("test-ext".to_string(), Duration::from_millis(100));
        assert!(result.success);
        assert_eq!(result.extension_id, "test-ext");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_reload_result_failure() {
        let result = ReloadResult::failure(
            "test-ext".to_string(),
            Duration::from_millis(50),
            "load error".to_string(),
        );
        assert!(!result.success);
        assert_eq!(result.extension_id, "test-ext");
        assert_eq!(result.error, Some("load error".to_string()));
    }

    // =============================================================================
    // Error Types
    // =============================================================================

    #[test]
    fn test_extension_manager_error_display() {
        let err = ExtensionManagerError::NotFound {
            id: "com.example.test".to_string(),
        };
        assert!(err.to_string().contains("com.example.test"));

        let err = ExtensionManagerError::NotEnabled {
            id: "com.example.disabled".to_string(),
        };
        assert!(err.to_string().contains("not enabled"));
    }
}
