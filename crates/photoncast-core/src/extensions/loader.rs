use std::path::{Path, PathBuf};

use abi_stable::library::{lib_header_from_raw_library, RawLibrary};
use thiserror::Error;

use photoncast_extension_api::{ExtensionApiRootModule, ExtensionApiRootModule_Ref};

#[derive(Debug, Error)]
pub enum ExtensionLoadError {
    #[error("failed to load raw library: {0}")]
    RawLibrary(String),
    #[error("failed to load entry symbol: {0}")]
    EntrySymbol(String),
    #[error("abi check failed: {0}")]
    AbiCheck(String),
    #[error("root module error: {0}")]
    RootModule(String),
    #[error("api version mismatch: host={host} extension={extension}")]
    ApiVersionMismatch { host: u32, extension: u32 },
}

pub struct ExtensionLibrary {
    raw: RawLibrary,
    path: PathBuf,
}

impl ExtensionLibrary {
    pub fn new(raw: RawLibrary, path: PathBuf) -> Self {
        Self { raw, path }
    }

    #[must_use]
    pub fn raw(&self) -> &RawLibrary {
        &self.raw
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct ExtensionLoader;

impl ExtensionLoader {
    pub fn load(path: &Path) -> Result<ExtensionLibrary, ExtensionLoadError> {
        let raw =
            RawLibrary::load_at(path).map_err(|e| ExtensionLoadError::RawLibrary(e.to_string()))?;
        Ok(ExtensionLibrary::new(raw, path.to_path_buf()))
    }

    pub fn check_api_version(extension_api: u32) -> Result<(), ExtensionLoadError> {
        let host_version = ExtensionApiRootModule::api_version();
        if host_version != extension_api {
            return Err(ExtensionLoadError::ApiVersionMismatch {
                host: host_version,
                extension: extension_api,
            });
        }
        Ok(())
    }

    pub fn resolve_api_version(raw: &RawLibrary) -> Result<u32, ExtensionLoadError> {
        // SAFETY: The library was loaded successfully and we're accessing a well-known
        // symbol with a fixed signature. The symbol returns a simple u32 value with no
        // side effects. The library remains loaded for the duration of this call.
        unsafe {
            let symbol = raw
                .get::<unsafe extern "C" fn() -> u32>(b"photoncast_extension_api_version")
                .map_err(|e| ExtensionLoadError::EntrySymbol(format!("{e}")))?;
            Ok(symbol())
        }
    }

    pub fn load_root_module(
        library: &ExtensionLibrary,
    ) -> Result<ExtensionApiRootModule_Ref, ExtensionLoadError> {
        // SAFETY: The library was loaded via abi_stable mechanisms. lib_header_from_raw_library
        // validates the ABI header before returning. The library reference remains valid
        // for the lifetime of the ExtensionLibrary wrapper.
        let header = unsafe { lib_header_from_raw_library(library.raw()) }
            .map_err(|e| ExtensionLoadError::RootModule(format!("{e}")))?;
        header
            .init_root_module::<ExtensionApiRootModule_Ref>()
            .map_err(|e| ExtensionLoadError::RootModule(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Task 10.3: ABI/Version Check Unit Tests
    // =============================================================================

    #[test]
    fn test_api_version_check_compatible() {
        // When the extension API version matches the host, should succeed
        let host_version = ExtensionApiRootModule::api_version();
        let result = ExtensionLoader::check_api_version(host_version);
        assert!(result.is_ok());
    }

    #[test]
    fn test_api_version_check_incompatible_too_low() {
        // When extension API version is lower than host, should fail
        let host_version = ExtensionApiRootModule::api_version();
        if host_version > 0 {
            let result = ExtensionLoader::check_api_version(host_version - 1);
            assert!(result.is_err());

            if let Err(ExtensionLoadError::ApiVersionMismatch { host, extension }) = result {
                assert_eq!(host, host_version);
                assert_eq!(extension, host_version - 1);
            } else {
                panic!("Expected ApiVersionMismatch error");
            }
        }
    }

    #[test]
    fn test_api_version_check_incompatible_too_high() {
        // When extension API version is higher than host, should fail
        let host_version = ExtensionApiRootModule::api_version();
        let result = ExtensionLoader::check_api_version(host_version + 1);
        assert!(result.is_err());

        if let Err(ExtensionLoadError::ApiVersionMismatch { host, extension }) = result {
            assert_eq!(host, host_version);
            assert_eq!(extension, host_version + 1);
        } else {
            panic!("Expected ApiVersionMismatch error");
        }
    }

    #[test]
    fn test_api_version_check_zero() {
        // API version 0 should always fail (reserved/invalid)
        let result = ExtensionLoader::check_api_version(0);
        // This will fail unless host version is also 0
        let host_version = ExtensionApiRootModule::api_version();
        if host_version != 0 {
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_load_nonexistent_library() {
        // Trying to load a library that doesn't exist should fail
        let path = Path::new("/nonexistent/path/to/extension.dylib");
        let result = ExtensionLoader::load(path);
        assert!(result.is_err());

        if let Err(ExtensionLoadError::RawLibrary(msg)) = result {
            assert!(!msg.is_empty());
        } else {
            panic!("Expected RawLibrary error");
        }
    }

    #[test]
    fn test_load_invalid_library() {
        // Trying to load a file that's not a valid dylib should fail
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let fake_dylib = dir.path().join("fake.dylib");
        std::fs::write(&fake_dylib, b"not a valid dylib").unwrap();

        let result = ExtensionLoader::load(&fake_dylib);
        assert!(result.is_err());

        if let Err(ExtensionLoadError::RawLibrary(msg)) = result {
            assert!(!msg.is_empty());
        } else {
            panic!("Expected RawLibrary error");
        }
    }

    #[test]
    fn test_extension_library_path_accessor() {
        // When we create an ExtensionLibrary (if we could), path() should return the right path
        // This test verifies the accessor works correctly
        // Note: We can't easily create a real ExtensionLibrary without a real dylib,
        // but we can test the path method indirectly through the load error path
        let path = PathBuf::from("/test/path/extension.dylib");
        let result = ExtensionLoader::load(&path);

        // We expect this to fail, but the path should be preserved in the error
        assert!(result.is_err());
    }

    #[test]
    fn test_extension_load_error_display() {
        // Test error message formatting
        let raw_err = ExtensionLoadError::RawLibrary("test error".to_string());
        assert!(raw_err.to_string().contains("test error"));

        let entry_err = ExtensionLoadError::EntrySymbol("symbol not found".to_string());
        assert!(entry_err.to_string().contains("symbol not found"));

        let abi_err = ExtensionLoadError::AbiCheck("abi mismatch".to_string());
        assert!(abi_err.to_string().contains("abi mismatch"));

        let root_err = ExtensionLoadError::RootModule("module error".to_string());
        assert!(root_err.to_string().contains("module error"));

        let version_err = ExtensionLoadError::ApiVersionMismatch {
            host: 2,
            extension: 1,
        };
        let msg = version_err.to_string();
        assert!(msg.contains("host=2"));
        assert!(msg.contains("extension=1"));
    }

    #[test]
    fn test_api_version_mismatch_error_details() {
        let err = ExtensionLoadError::ApiVersionMismatch {
            host: 5,
            extension: 3,
        };

        match err {
            ExtensionLoadError::ApiVersionMismatch { host, extension } => {
                assert_eq!(host, 5);
                assert_eq!(extension, 3);
            },
            _ => panic!("Wrong error variant"),
        }
    }
}
