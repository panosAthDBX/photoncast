use std::path::PathBuf;
use std::sync::Arc;

use abi_stable::std_types::{ROption, RString, RVec};
use parking_lot::{Mutex, RwLock};
use photoncast_extension_api::RStr;
use photoncast_extension_api::{
    ExtensionApiError, ExtensionApiResult, ExtensionStorage, ExtensionStorageTrait,
    PreferenceDefinition, PreferenceKind, PreferenceStore, PreferenceStoreTrait, PreferenceValue,
    PreferenceValues, SelectOption,
};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::extensions::manifest::{PreferenceManifest, SelectOption as ManifestSelectOption};

#[derive(Debug, Error)]
pub enum ExtensionStorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct ExtensionStorageImpl {
    // Using Mutex instead of RwLock because rusqlite::Connection is Send but not Sync.
    // Mutex<T>: Sync only requires T: Send, so this makes ExtensionStorageImpl
    // automatically Send + Sync without needing unsafe impls.
    conn: Arc<Mutex<Connection>>,
    namespace: String,
}

impl ExtensionStorageImpl {
    pub fn new(path: PathBuf, namespace: impl Into<String>) -> Result<Self, ExtensionStorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS extension_storage (
                namespace TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY(namespace, key)
            );
            CREATE INDEX IF NOT EXISTS idx_extension_storage_namespace ON extension_storage(namespace);
            ",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            namespace: namespace.into(),
        })
    }

    #[must_use]
    pub fn api_handle(&self) -> ExtensionStorage {
        ExtensionStorage::new(self.clone())
    }
}

impl ExtensionStorageTrait for ExtensionStorageImpl {
    fn get(&self, key: RStr<'_>) -> ExtensionApiResult<ROption<RString>> {
        let conn = self.conn.lock();
        let value: Option<String> = match conn
            .query_row(
                "SELECT value FROM extension_storage WHERE namespace = ?1 AND key = ?2",
                params![self.namespace, key.as_str()],
                |row| row.get(0),
            )
            .optional()
        {
            Ok(value) => value,
            Err(e) => return Err(ExtensionApiError::message(e.to_string())).into(),
        };
        Ok(value.map(RString::from).into()).into()
    }

    fn set(&self, key: RStr<'_>, value: RStr<'_>) -> ExtensionApiResult<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().timestamp();
        if let Err(e) = conn.execute(
            "INSERT INTO extension_storage (namespace, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)\
             ON CONFLICT(namespace, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![self.namespace, key.as_str(), value.as_str(), now],
        ) {
            return Err(ExtensionApiError::message(e.to_string())).into();
        }
        Ok(()).into()
    }

    fn delete(&self, key: RStr<'_>) -> ExtensionApiResult<()> {
        let conn = self.conn.lock();
        if let Err(e) = conn.execute(
            "DELETE FROM extension_storage WHERE namespace = ?1 AND key = ?2",
            params![self.namespace, key.as_str()],
        ) {
            return Err(ExtensionApiError::message(e.to_string())).into();
        }
        Ok(()).into()
    }

    fn list(&self) -> ExtensionApiResult<RVec<RString>> {
        let conn = self.conn.lock();
        let mut stmt = match conn
            .prepare("SELECT key FROM extension_storage WHERE namespace = ?1 ORDER BY key ASC")
        {
            Ok(stmt) => stmt,
            Err(e) => return Err(ExtensionApiError::message(e.to_string())).into(),
        };
        let keys = match stmt.query_map(params![self.namespace], |row| row.get::<_, String>(0)) {
            Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
                Ok(items) => items,
                Err(e) => return Err(ExtensionApiError::message(e.to_string())).into(),
            },
            Err(e) => return Err(ExtensionApiError::message(e.to_string())).into(),
        };
        Ok(keys.into_iter().map(RString::from).collect()).into()
    }
}

#[derive(Clone)]
pub struct PreferenceStoreImpl {
    values: Arc<RwLock<Vec<(String, PreferenceValue)>>>,
    definitions: Arc<RwLock<Vec<PreferenceDefinition>>>,
}

impl PreferenceStoreImpl {
    pub fn new(definitions: Vec<PreferenceManifest>) -> Self {
        let defs = definitions
            .into_iter()
            .map(|pref| PreferenceDefinition {
                name: pref.name.into(),
                title: pref.title.into(),
                description: pref.description.map(RString::from).into(),
                required: pref.required,
                kind: to_preference_kind(pref.kind, pref.options),
                default_value: pref
                    .default
                    .map(|v| serde_json::to_string(&v).unwrap_or_default())
                    .map(RString::from)
                    .map(PreferenceValue::String)
                    .into(),
            })
            .collect();

        Self {
            values: Arc::new(RwLock::new(Vec::new())),
            definitions: Arc::new(RwLock::new(defs)),
        }
    }

    pub fn values(&self) -> ExtensionApiResult<PreferenceValues> {
        let values = self
            .values
            .read()
            .iter()
            .map(|(k, v)| abi_stable::std_types::Tuple2(RString::from(k.clone()), v.clone()))
            .collect();
        Ok(PreferenceValues { values }).into()
    }

    pub fn api_handle(&self) -> PreferenceStore {
        PreferenceStore::new(self.clone())
    }
}

impl PreferenceStoreTrait for PreferenceStoreImpl {
    fn get(&self, key: RStr<'_>) -> ExtensionApiResult<ROption<PreferenceValue>> {
        let values = self.values.read();
        let value = values
            .iter()
            .find(|(k, _)| k == key.as_str())
            .map(|(_, v)| v.clone());
        Ok(value.into()).into()
    }

    fn set(&self, key: RStr<'_>, value: PreferenceValue) -> ExtensionApiResult<()> {
        let key = key.as_str();
        let mut values = self.values.write();
        if let Some(existing) = values.iter_mut().find(|(k, _)| k == key) {
            existing.1 = value;
        } else {
            values.push((key.to_string(), value));
        }
        Ok(()).into()
    }

    #[allow(clippy::needless_pass_by_value)]
    fn definitions(&self) -> RVec<PreferenceDefinition> {
        self.definitions.read().clone().into()
    }
}

#[allow(clippy::match_same_arms, clippy::needless_pass_by_value)]
fn to_preference_kind(kind: String, options: Vec<ManifestSelectOption>) -> PreferenceKind {
    match kind.as_str() {
        "string" => PreferenceKind::String,
        "number" => PreferenceKind::Number,
        "boolean" => PreferenceKind::Boolean,
        "secret" => PreferenceKind::Secret,
        "select" => PreferenceKind::Select {
            options: options
                .into_iter()
                .map(|opt| SelectOption {
                    label: opt.label.into(),
                    value: opt.value.into(),
                })
                .collect(),
        },
        "file" => PreferenceKind::File,
        "directory" => PreferenceKind::Directory,
        _ => PreferenceKind::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abi_stable::std_types::RResult;

    #[test]
    fn test_storage_new_creates_database() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test_storage.db");
        let storage = ExtensionStorageImpl::new(db_path.clone(), "test-ext");
        assert!(storage.is_ok());
        assert!(db_path.exists());
    }

    #[test]
    fn test_storage_get_missing_key_returns_none() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test_storage.db");
        let storage = ExtensionStorageImpl::new(db_path, "test-ext").expect("failed to create storage");

        let result = storage.get(RStr::from_str("nonexistent"));
        match result {
            RResult::ROk(val) => {
                assert!(val.is_none(), "Expected None for missing key");
            },
            RResult::RErr(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_storage_set_and_get_roundtrip() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test_storage.db");
        let storage = ExtensionStorageImpl::new(db_path, "test-ext").expect("failed to create storage");

        // Set a value
        let set_result = storage.set(RStr::from_str("my_key"), RStr::from_str("my_value"));
        assert!(set_result.is_ok(), "set() should succeed");

        // Get the value back
        let get_result = storage.get(RStr::from_str("my_key"));
        match get_result {
            RResult::ROk(val) => {
                let val = val.into_option().expect("Expected Some value");
                assert_eq!(val.as_str(), "my_value");
            },
            RResult::RErr(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_storage_set_overwrites_existing() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test_storage.db");
        let storage = ExtensionStorageImpl::new(db_path, "test-ext").expect("failed to create storage");

        storage.set(RStr::from_str("key"), RStr::from_str("value1"));
        storage.set(RStr::from_str("key"), RStr::from_str("value2"));

        match storage.get(RStr::from_str("key")) {
            RResult::ROk(val) => {
                let val = val.into_option().expect("Expected Some value");
                assert_eq!(val.as_str(), "value2");
            },
            RResult::RErr(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_storage_delete() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test_storage.db");
        let storage = ExtensionStorageImpl::new(db_path, "test-ext").expect("failed to create storage");

        storage.set(RStr::from_str("key"), RStr::from_str("value"));
        storage.delete(RStr::from_str("key"));

        match storage.get(RStr::from_str("key")) {
            RResult::ROk(val) => {
                assert!(val.is_none(), "Expected None after delete");
            },
            RResult::RErr(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_storage_list_keys() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test_storage.db");
        let storage = ExtensionStorageImpl::new(db_path, "test-ext").expect("failed to create storage");

        storage.set(RStr::from_str("beta"), RStr::from_str("v1"));
        storage.set(RStr::from_str("alpha"), RStr::from_str("v2"));

        match storage.list() {
            RResult::ROk(keys) => {
                let keys: Vec<&str> = keys.iter().map(photoncast_extension_api::RString::as_str).collect();
                assert_eq!(keys, vec!["alpha", "beta"]); // sorted
            },
            RResult::RErr(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_storage_namespace_isolation() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test_storage.db");
        let storage_a = ExtensionStorageImpl::new(db_path.clone(), "ext-a").expect("failed to create storage");
        let storage_b = ExtensionStorageImpl::new(db_path, "ext-b").expect("failed to create storage");

        storage_a.set(RStr::from_str("key"), RStr::from_str("from_a"));
        storage_b.set(RStr::from_str("key"), RStr::from_str("from_b"));

        match storage_a.get(RStr::from_str("key")) {
            RResult::ROk(val) => {
                assert_eq!(val.into_option().unwrap().as_str(), "from_a");
            },
            RResult::RErr(e) => panic!("Unexpected error: {e:?}"),
        }
        match storage_b.get(RStr::from_str("key")) {
            RResult::ROk(val) => {
                assert_eq!(val.into_option().unwrap().as_str(), "from_b");
            },
            RResult::RErr(e) => panic!("Unexpected error: {e:?}"),
        }
    }
}
