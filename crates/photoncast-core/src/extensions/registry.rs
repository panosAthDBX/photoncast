use std::collections::HashMap;

use thiserror::Error;

use crate::extensions::manifest::ExtensionManifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionState {
    Discovered,
    Loaded,
    Active,
    Disabled,
    Failed,
    Unloaded,
}

impl ExtensionState {
    #[must_use]
    pub const fn can_transition(self, next: ExtensionState) -> bool {
        matches!(
            (self, next),
            (ExtensionState::Discovered | ExtensionState::Disabled,
ExtensionState::Loaded) |
(ExtensionState::Loaded,
ExtensionState::Active | ExtensionState::Failed | ExtensionState::Unloaded) |
(ExtensionState::Active, ExtensionState::Failed | ExtensionState::Disabled) |
(ExtensionState::Disabled, ExtensionState::Unloaded) |
(ExtensionState::Failed, ExtensionState::Disabled)
        )
    }
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidTransition {
        from: ExtensionState,
        to: ExtensionState,
    },
    #[error("extension not found: {id}")]
    NotFound { id: String },
}

#[derive(Debug, Clone)]
pub struct ExtensionRecord {
    pub manifest: ExtensionManifest,
    pub enabled: bool,
    pub state: ExtensionState,
    pub last_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    records: HashMap<String, ExtensionRecord>,
}

impl ExtensionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, manifest: ExtensionManifest, enabled: bool) {
        let id = manifest.extension.id.clone();
        let state = if enabled {
            ExtensionState::Discovered
        } else {
            ExtensionState::Disabled
        };
        self.records.insert(
            id,
            ExtensionRecord {
                manifest,
                enabled,
                state,
                last_error: None,
            },
        );
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), RegistryError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| RegistryError::NotFound { id: id.to_string() })?;
        record.enabled = enabled;
        record.state = if enabled {
            ExtensionState::Discovered
        } else {
            ExtensionState::Disabled
        };
        Ok(())
    }

    pub fn update_state(&mut self, id: &str, next: ExtensionState) -> Result<(), RegistryError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| RegistryError::NotFound { id: id.to_string() })?;
        if !record.state.can_transition(next) {
            return Err(RegistryError::InvalidTransition {
                from: record.state,
                to: next,
            });
        }
        record.state = next;
        Ok(())
    }

    pub fn set_error(&mut self, id: &str, error: Option<String>) {
        if let Some(record) = self.records.get_mut(id) {
            record.last_error = error;
        }
    }

    pub fn get(&self, id: &str) -> Option<&ExtensionRecord> {
        self.records.get(id)
    }

    pub fn list(&self) -> Vec<&ExtensionRecord> {
        self.records.values().collect()
    }

    pub fn remove(&mut self, id: &str) {
        self.records.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::manifest::{
        ExtensionEntry, ExtensionInfo, ExtensionManifest, Permissions, SUPPORTED_API_VERSION,
    };

    fn create_test_manifest(id: &str) -> ExtensionManifest {
        ExtensionManifest {
            schema_version: 1,
            extension: ExtensionInfo {
                id: id.to_string(),
                name: "Test Extension".to_string(),
                version: "1.0.0".to_string(),
                description: "Test description".to_string(),
                author: None,
                license: None,
                homepage: None,
                min_photoncast_version: None,
                api_version: SUPPORTED_API_VERSION,
            },
            entry: ExtensionEntry {
                kind: "dylib".to_string(),
                path: "test.dylib".to_string(),
            },
            permissions: Permissions::default(),
            commands: vec![],
            preferences: vec![],
            directory: None,
        }
    }

    // =============================================================================
    // Task 10.4: Lifecycle State Transition Unit Tests
    // =============================================================================

    #[test]
    fn test_state_transition_discovered_to_loaded() {
        assert!(ExtensionState::Discovered.can_transition(ExtensionState::Loaded));
    }

    #[test]
    fn test_state_transition_loaded_to_active() {
        assert!(ExtensionState::Loaded.can_transition(ExtensionState::Active));
    }

    #[test]
    fn test_state_transition_loaded_to_failed() {
        assert!(ExtensionState::Loaded.can_transition(ExtensionState::Failed));
    }

    #[test]
    fn test_state_transition_active_to_failed() {
        assert!(ExtensionState::Active.can_transition(ExtensionState::Failed));
    }

    #[test]
    fn test_state_transition_active_to_disabled() {
        assert!(ExtensionState::Active.can_transition(ExtensionState::Disabled));
    }

    #[test]
    fn test_state_transition_disabled_to_unloaded() {
        assert!(ExtensionState::Disabled.can_transition(ExtensionState::Unloaded));
    }

    #[test]
    fn test_state_transition_loaded_to_unloaded() {
        assert!(ExtensionState::Loaded.can_transition(ExtensionState::Unloaded));
    }

    #[test]
    fn test_state_transition_failed_to_disabled() {
        assert!(ExtensionState::Failed.can_transition(ExtensionState::Disabled));
    }

    #[test]
    fn test_state_transition_disabled_to_loaded() {
        assert!(ExtensionState::Disabled.can_transition(ExtensionState::Loaded));
    }

    #[test]
    fn test_invalid_transition_discovered_to_active() {
        // Cannot jump directly from Discovered to Active
        assert!(!ExtensionState::Discovered.can_transition(ExtensionState::Active));
    }

    #[test]
    fn test_invalid_transition_discovered_to_unloaded() {
        assert!(!ExtensionState::Discovered.can_transition(ExtensionState::Unloaded));
    }

    #[test]
    fn test_invalid_transition_unloaded_to_active() {
        assert!(!ExtensionState::Unloaded.can_transition(ExtensionState::Active));
    }

    #[test]
    fn test_invalid_transition_failed_to_active() {
        // Must go through Disabled -> Loaded first
        assert!(!ExtensionState::Failed.can_transition(ExtensionState::Active));
    }

    #[test]
    fn test_registry_insert_enabled() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest.clone(), true);

        let record = registry.get("com.example.test").unwrap();
        assert!(record.enabled);
        assert_eq!(record.state, ExtensionState::Discovered);
    }

    #[test]
    fn test_registry_insert_disabled() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest.clone(), false);

        let record = registry.get("com.example.test").unwrap();
        assert!(!record.enabled);
        assert_eq!(record.state, ExtensionState::Disabled);
    }

    #[test]
    fn test_registry_set_enabled() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest, false);
        let result = registry.set_enabled("com.example.test", true);

        assert!(result.is_ok());
        let record = registry.get("com.example.test").unwrap();
        assert!(record.enabled);
        assert_eq!(record.state, ExtensionState::Discovered);
    }

    #[test]
    fn test_registry_set_enabled_not_found() {
        let mut registry = ExtensionRegistry::new();

        let result = registry.set_enabled("nonexistent", true);
        assert!(result.is_err());

        if let Err(RegistryError::NotFound { id }) = result {
            assert_eq!(id, "nonexistent");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_registry_update_state_valid_transition() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest, true);

        // Discovered -> Loaded
        let result = registry.update_state("com.example.test", ExtensionState::Loaded);
        assert!(result.is_ok());

        let record = registry.get("com.example.test").unwrap();
        assert_eq!(record.state, ExtensionState::Loaded);

        // Loaded -> Active
        let result = registry.update_state("com.example.test", ExtensionState::Active);
        assert!(result.is_ok());

        let record = registry.get("com.example.test").unwrap();
        assert_eq!(record.state, ExtensionState::Active);
    }

    #[test]
    fn test_registry_update_state_invalid_transition() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest, true); // State: Discovered

        // Try invalid transition: Discovered -> Active (should fail)
        let result = registry.update_state("com.example.test", ExtensionState::Active);
        assert!(result.is_err());

        if let Err(RegistryError::InvalidTransition { from, to }) = result {
            assert_eq!(from, ExtensionState::Discovered);
            assert_eq!(to, ExtensionState::Active);
        } else {
            panic!("Expected InvalidTransition error");
        }

        // State should remain unchanged
        let record = registry.get("com.example.test").unwrap();
        assert_eq!(record.state, ExtensionState::Discovered);
    }

    #[test]
    fn test_registry_update_state_not_found() {
        let mut registry = ExtensionRegistry::new();

        let result = registry.update_state("nonexistent", ExtensionState::Loaded);
        assert!(result.is_err());

        if let Err(RegistryError::NotFound { id }) = result {
            assert_eq!(id, "nonexistent");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_registry_set_and_get_error() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest, true);

        // Set error
        registry.set_error("com.example.test", Some("Test error message".to_string()));

        let record = registry.get("com.example.test").unwrap();
        assert_eq!(record.last_error, Some("Test error message".to_string()));

        // Clear error
        registry.set_error("com.example.test", None);

        let record = registry.get("com.example.test").unwrap();
        assert_eq!(record.last_error, None);
    }

    #[test]
    fn test_registry_set_error_nonexistent() {
        let mut registry = ExtensionRegistry::new();

        // Should not panic, just no-op
        registry.set_error("nonexistent", Some("error".to_string()));
    }

    #[test]
    fn test_registry_list() {
        let mut registry = ExtensionRegistry::new();

        registry.insert(create_test_manifest("com.example.ext1"), true);
        registry.insert(create_test_manifest("com.example.ext2"), false);
        registry.insert(create_test_manifest("com.example.ext3"), true);

        let records = registry.list();
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn test_registry_remove() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest, true);
        assert!(registry.get("com.example.test").is_some());

        registry.remove("com.example.test");
        assert!(registry.get("com.example.test").is_none());
    }

    #[test]
    fn test_full_lifecycle_enabled() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        // Insert as enabled -> Discovered
        registry.insert(manifest, true);
        assert_eq!(
            registry.get("com.example.test").unwrap().state,
            ExtensionState::Discovered
        );

        // Discovered -> Loaded
        registry
            .update_state("com.example.test", ExtensionState::Loaded)
            .unwrap();
        assert_eq!(
            registry.get("com.example.test").unwrap().state,
            ExtensionState::Loaded
        );

        // Loaded -> Active
        registry
            .update_state("com.example.test", ExtensionState::Active)
            .unwrap();
        assert_eq!(
            registry.get("com.example.test").unwrap().state,
            ExtensionState::Active
        );

        // Active -> Disabled (deactivate)
        registry
            .update_state("com.example.test", ExtensionState::Disabled)
            .unwrap();
        assert_eq!(
            registry.get("com.example.test").unwrap().state,
            ExtensionState::Disabled
        );

        // Disabled -> Unloaded
        registry
            .update_state("com.example.test", ExtensionState::Unloaded)
            .unwrap();
        assert_eq!(
            registry.get("com.example.test").unwrap().state,
            ExtensionState::Unloaded
        );
    }

    #[test]
    fn test_failure_recovery_lifecycle() {
        let mut registry = ExtensionRegistry::new();
        let manifest = create_test_manifest("com.example.test");

        registry.insert(manifest, true);

        // Discovered -> Loaded
        registry
            .update_state("com.example.test", ExtensionState::Loaded)
            .unwrap();

        // Loaded -> Failed (simulating load error)
        registry
            .update_state("com.example.test", ExtensionState::Failed)
            .unwrap();
        registry.set_error(
            "com.example.test",
            Some("Load failed: missing symbol".to_string()),
        );

        assert_eq!(
            registry.get("com.example.test").unwrap().state,
            ExtensionState::Failed
        );
        assert!(registry
            .get("com.example.test")
            .unwrap()
            .last_error
            .is_some());

        // Failed -> Disabled (user disables the broken extension)
        registry
            .update_state("com.example.test", ExtensionState::Disabled)
            .unwrap();

        // Disabled -> Loaded (try again after fix)
        registry
            .update_state("com.example.test", ExtensionState::Loaded)
            .unwrap();
        registry.set_error("com.example.test", None); // Clear error

        assert_eq!(
            registry.get("com.example.test").unwrap().state,
            ExtensionState::Loaded
        );
        assert!(registry
            .get("com.example.test")
            .unwrap()
            .last_error
            .is_none());
    }

    #[test]
    fn test_registry_error_display() {
        let invalid_err = RegistryError::InvalidTransition {
            from: ExtensionState::Discovered,
            to: ExtensionState::Active,
        };
        let msg = invalid_err.to_string();
        assert!(msg.contains("invalid state transition"));
        assert!(msg.contains("Discovered"));
        assert!(msg.contains("Active"));

        let not_found_err = RegistryError::NotFound {
            id: "com.example.missing".to_string(),
        };
        let msg = not_found_err.to_string();
        assert!(msg.contains("not found"));
        assert!(msg.contains("com.example.missing"));
    }
}
