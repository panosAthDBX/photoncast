# Native objc2 Spotlight Integration Specification

**Author:** Droid  
**Created:** 2026-01-22  
**Status:** Draft  
**Estimated Duration:** 3-4 weeks  
**Dependencies:** objc2 0.6.x, objc2-foundation 0.3.x, block2 0.5.x

---

## 1. Overview

### 1.1 Purpose

Replace the current `mdfind` CLI-based Spotlight integration with native macOS APIs using the objc2 Rust crates. This will provide:

1. **Better Performance** - Direct API access eliminates process spawn overhead
2. **Richer Results** - Access to full metadata attributes in-flight
3. **Live Updates** - NSMetadataQuery can notify on file system changes
4. **Advanced Queries** - Build complex predicates programmatically
5. **Cancellation** - Proper async cancellation support

### 1.2 Current Implementation

The existing implementation in `crates/photoncast-core/src/platform/spotlight.rs` uses:
- `mdfind` CLI via `std::process::Command` / `tokio::process::Command`
- Simple name-based queries (`mdfind -name "query"`)
- Path-only results with lazy metadata loading
- ~500ms timeout per query

**Limitations:**
- Process spawn overhead (~10-20ms per query)
- No access to rich metadata without additional `mdls` calls
- No live update support
- Limited predicate expressiveness

### 1.3 Target State

Native Spotlight integration using:
- `objc2-foundation` for `NSMetadataQuery`, `NSPredicate`, `NSNotification`
- `block2` for Objective-C block callbacks
- Async/await friendly API with proper cancellation
- Rich metadata access (size, dates, content type, etc.)
- Optional live monitoring for file changes

---

## 2. Architecture

### 2.1 Module Structure

```
crates/photoncast-core/src/search/
├── mod.rs                    # Export new modules
├── spotlight/
│   ├── mod.rs               # Public API exports
│   ├── query.rs             # NSMetadataQuery wrapper
│   ├── predicate.rs         # NSPredicate builder
│   ├── attributes.rs        # Metadata attribute constants
│   └── result.rs            # FileResult with rich metadata
├── file_index.rs            # Keep as fallback (no changes)
├── file_query.rs            # Query parser (minimal changes)
└── ...
```

### 2.2 Component Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                        FileSearchView                         │
│                    (crates/photoncast/src)                   │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│                    SpotlightSearchService                     │
│              (new: search/spotlight/mod.rs)                  │
├──────────────────────────────────────────────────────────────┤
│ • search_files(query, filter) -> Vec<FileResult>             │
│ • search_with_options(SpotlightSearchOptions)                │
│ • start_live_query() / stop_live_query()                     │
└──────────────────────────┬───────────────────────────────────┘
                           │
           ┌───────────────┴───────────────┐
           │                               │
           ▼                               ▼
┌─────────────────────────┐    ┌─────────────────────────┐
│   NSMetadataQueryWrapper │    │   PredicateBuilder      │
│   (query.rs)             │    │   (predicate.rs)        │
├─────────────────────────┤    ├─────────────────────────┤
│ • new() -> Self          │    │ • name_contains()       │
│ • set_predicate()        │    │ • extension_is()        │
│ • set_search_scopes()    │    │ • content_type()        │
│ • execute_async()        │    │ • modified_after()      │
│ • execute_sync()         │    │ • and() / or()          │
└─────────────────────────┘    └─────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│                    objc2-foundation                          │
│    NSMetadataQuery, NSPredicate, NSNotificationCenter        │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 Data Flow

```
1. User types query in FileSearchView
                    │
                    ▼
2. FileQuery::parse() → FileQuery struct (terms, filters, location)
                    │
                    ▼
3. PredicateBuilder converts FileQuery → NSPredicate
   • name terms → kMDItemFSName CONTAINS[cd] "term"
   • .pdf filter → kMDItemFSName == "*.pdf"c
   • in:downloads → search scope = ~/Downloads
                    │
                    ▼
4. NSMetadataQueryWrapper executes query
   • Sets predicate, scopes, sort descriptors
   • Starts query on background thread
   • Waits for NSMetadataQueryDidFinishGatheringNotification
                    │
                    ▼
5. Results extracted from NSMetadataQuery.results
   • Map NSMetadataItem → FileResult
   • Extract attributes (path, name, size, dates, type)
                    │
                    ▼
6. Vec<FileResult> returned to UI
```

---

## 3. Detailed Design

### 3.1 SpotlightSearchService (Public API)

```rust
// crates/photoncast-core/src/search/spotlight/mod.rs

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for Spotlight searches.
#[derive(Debug, Clone)]
pub struct SpotlightSearchOptions {
    /// Maximum results to return.
    pub max_results: usize,
    /// Search timeout.
    pub timeout: Duration,
    /// Directories to search (defaults to user home).
    pub search_scopes: Vec<PathBuf>,
    /// File type filter (UTI-based).
    pub content_type_filter: Option<ContentTypeFilter>,
    /// Sort order for results.
    pub sort_by: SortOrder,
}

impl Default for SpotlightSearchOptions {
    fn default() -> Self {
        Self {
            max_results: 50,
            timeout: Duration::from_millis(500),
            search_scopes: default_search_scopes(),
            content_type_filter: None,
            sort_by: SortOrder::RelevanceThenDate,
        }
    }
}

/// Content type filter (maps to UTI types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentTypeFilter {
    Documents,
    Images,
    Videos,
    Audio,
    Archives,
    Code,
    Folders,
}

/// Sort order for search results.
#[derive(Debug, Clone, Copy, Default)]
pub enum SortOrder {
    #[default]
    RelevanceThenDate,
    DateDescending,
    NameAscending,
    SizeDescending,
}

/// Main service for Spotlight searches.
pub struct SpotlightSearchService {
    options: SpotlightSearchOptions,
}

impl SpotlightSearchService {
    /// Creates a new service with default options.
    pub fn new() -> Self { ... }

    /// Creates with custom options.
    pub fn with_options(options: SpotlightSearchOptions) -> Self { ... }

    /// Searches files by name (simple API).
    pub async fn search_files(&self, query: &str) -> Result<Vec<FileResult>, SpotlightError> { ... }

    /// Searches with full FileQuery support.
    pub async fn search_with_query(&self, query: &FileQuery) -> Result<Vec<FileResult>, SpotlightError> { ... }

    /// Synchronous search (for compatibility).
    pub fn search_files_sync(&self, query: &str) -> Result<Vec<FileResult>, SpotlightError> { ... }
}
```

### 3.2 NSMetadataQueryWrapper (Internal)

```rust
// crates/photoncast-core/src/search/spotlight/query.rs

use objc2::rc::Retained;
use objc2_foundation::{
    NSArray, NSMetadataQuery, NSMetadataQueryDidFinishGatheringNotification,
    NSNotificationCenter, NSPredicate, NSString, NSURL,
};
use block2::RcBlock;

/// Wraps NSMetadataQuery for safe Rust usage.
pub(crate) struct MetadataQueryWrapper {
    query: Retained<NSMetadataQuery>,
    /// Channel for completion notification.
    completion_rx: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl MetadataQueryWrapper {
    /// Creates a new query wrapper.
    pub fn new() -> Self {
        // SAFETY: NSMetadataQuery::new() is safe to call
        let query = unsafe { NSMetadataQuery::new() };
        Self {
            query,
            completion_rx: None,
        }
    }

    /// Sets the search predicate.
    pub fn set_predicate(&self, predicate: &NSPredicate) {
        unsafe { self.query.setPredicate(Some(predicate)) };
    }

    /// Sets search scopes (directories to search).
    pub fn set_search_scopes(&self, scopes: &[PathBuf]) {
        let urls: Vec<Retained<NSURL>> = scopes
            .iter()
            .filter_map(|p| {
                let path_str = NSString::from_str(&p.to_string_lossy());
                unsafe { NSURL::fileURLWithPath(&path_str) }
            })
            .collect();

        let array = NSArray::from_vec(urls);
        unsafe { self.query.setSearchScopes(&array) };
    }

    /// Starts the query and returns results when complete.
    pub async fn execute(&mut self) -> Result<Vec<MetadataItem>, SpotlightError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.completion_rx = Some(rx);

        // Register for completion notification
        self.register_completion_observer(tx);

        // Start the query
        let started = unsafe { self.query.startQuery() };
        if !started {
            return Err(SpotlightError::QueryStartFailed);
        }

        // Wait for completion with timeout
        match tokio::time::timeout(Duration::from_millis(500), self.completion_rx.take().unwrap()).await {
            Ok(_) => self.collect_results(),
            Err(_) => {
                unsafe { self.query.stopQuery() };
                Err(SpotlightError::Timeout { timeout_ms: 500 })
            }
        }
    }

    /// Collects results from the completed query.
    fn collect_results(&self) -> Result<Vec<MetadataItem>, SpotlightError> {
        let count = unsafe { self.query.resultCount() };
        let mut results = Vec::with_capacity(count as usize);

        for i in 0..count {
            let item = unsafe { self.query.resultAtIndex(i) };
            if let Some(metadata_item) = MetadataItem::from_objc(&item) {
                results.push(metadata_item);
            }
        }

        Ok(results)
    }

    /// Registers observer for query completion.
    fn register_completion_observer(&self, tx: tokio::sync::oneshot::Sender<()>) {
        // Create block for notification callback
        let block = RcBlock::new(move |_notification: &objc2_foundation::NSNotification| {
            let _ = tx.send(());
        });

        let center = unsafe { NSNotificationCenter::defaultCenter() };
        unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSMetadataQueryDidFinishGatheringNotification),
                Some(&*self.query),
                None, // Use default queue
                &block,
            );
        }
    }
}

impl Drop for MetadataQueryWrapper {
    fn drop(&mut self) {
        // Stop query if still running
        unsafe {
            if self.query.isStarted() && !self.query.isStopped() {
                self.query.stopQuery();
            }
        }
    }
}
```

### 3.3 PredicateBuilder

```rust
// crates/photoncast-core/src/search/spotlight/predicate.rs

use objc2::rc::Retained;
use objc2_foundation::{NSPredicate, NSString};

/// Metadata attribute keys (kMDItem* constants).
pub mod attributes {
    pub const FS_NAME: &str = "kMDItemFSName";
    pub const DISPLAY_NAME: &str = "kMDItemDisplayName";
    pub const CONTENT_TYPE: &str = "kMDItemContentType";
    pub const CONTENT_TYPE_TREE: &str = "kMDItemContentTypeTree";
    pub const FS_SIZE: &str = "kMDItemFSSize";
    pub const CONTENT_MODIFICATION_DATE: &str = "kMDItemContentModificationDate";
    pub const CONTENT_CREATION_DATE: &str = "kMDItemContentCreationDate";
    pub const PATH: &str = "kMDItemPath";
}

/// Builder for creating NSPredicate for Spotlight queries.
pub struct PredicateBuilder {
    predicates: Vec<String>,
}

impl PredicateBuilder {
    pub fn new() -> Self {
        Self { predicates: Vec::new() }
    }

    /// Adds a name contains predicate (case-insensitive, diacritic-insensitive).
    pub fn name_contains(mut self, term: &str) -> Self {
        let escaped = Self::escape_value(term);
        self.predicates.push(format!(
            "{} CONTAINS[cd] \"{}\"",
            attributes::DISPLAY_NAME,
            escaped
        ));
        self
    }

    /// Adds a name prefix match.
    pub fn name_begins_with(mut self, prefix: &str) -> Self {
        let escaped = Self::escape_value(prefix);
        self.predicates.push(format!(
            "{} BEGINSWITH[cd] \"{}\"",
            attributes::DISPLAY_NAME,
            escaped
        ));
        self
    }

    /// Adds an extension filter (e.g., "pdf").
    pub fn extension_equals(mut self, ext: &str) -> Self {
        let escaped = Self::escape_value(ext);
        self.predicates.push(format!(
            "{} == \"*.{}\"c",
            attributes::FS_NAME,
            escaped
        ));
        self
    }

    /// Adds a content type filter (UTI).
    pub fn content_type(mut self, uti: &str) -> Self {
        self.predicates.push(format!(
            "{} == \"{}\"",
            attributes::CONTENT_TYPE,
            uti
        ));
        self
    }

    /// Adds a content type tree filter (matches UTI and conforming types).
    pub fn content_type_tree(mut self, uti: &str) -> Self {
        self.predicates.push(format!(
            "{} == \"{}\"",
            attributes::CONTENT_TYPE_TREE,
            uti
        ));
        self
    }

    /// Filters to folders only.
    pub fn folders_only(mut self) -> Self {
        self.predicates.push(format!(
            "{} == \"public.folder\"",
            attributes::CONTENT_TYPE
        ));
        self
    }

    /// Adds a modified date filter (after given date).
    pub fn modified_after(mut self, timestamp: i64) -> Self {
        self.predicates.push(format!(
            "{} >= $time.iso({})",
            attributes::CONTENT_MODIFICATION_DATE,
            timestamp
        ));
        self
    }

    /// Builds the final NSPredicate.
    pub fn build(self) -> Result<Retained<NSPredicate>, SpotlightError> {
        if self.predicates.is_empty() {
            // Match all files
            return Self::predicate_from_string("kMDItemFSName == '*'");
        }

        let combined = self.predicates.join(" && ");
        Self::predicate_from_string(&combined)
    }

    /// Creates an NSPredicate from a format string.
    fn predicate_from_string(format: &str) -> Result<Retained<NSPredicate>, SpotlightError> {
        let format_str = NSString::from_str(format);
        
        // SAFETY: predicateWithFormat: is safe if format string is valid
        let predicate = unsafe {
            NSPredicate::predicateWithFormat(&format_str)
        };
        
        Ok(predicate)
    }

    /// Escapes special characters in predicate values.
    fn escape_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('*', "\\*")
            .replace('?', "\\?")
    }
}

/// Converts FileQuery to NSPredicate.
pub fn file_query_to_predicate(query: &FileQuery) -> Result<Retained<NSPredicate>, SpotlightError> {
    let mut builder = PredicateBuilder::new();

    // Add name search terms
    for term in &query.terms {
        builder = builder.name_contains(term);
    }

    // Add exact phrase if present
    if let Some(phrase) = &query.exact_phrase {
        builder = builder.name_contains(phrase);
    }

    // Add file type filter
    if let Some(filter) = &query.file_type {
        match filter {
            FileTypeFilter::Extension(ext) => {
                builder = builder.extension_equals(ext);
            }
            FileTypeFilter::Category(category) => {
                for uti in category.uti_types() {
                    builder = builder.content_type_tree(uti);
                }
            }
        }
    }

    // Handle folder prioritization
    if query.prioritize_folders {
        builder = builder.folders_only();
    }

    builder.build()
}
```

### 3.4 MetadataItem (Rich Result)

```rust
// crates/photoncast-core/src/search/spotlight/result.rs

use std::path::PathBuf;
use std::time::SystemTime;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

/// Metadata extracted from a Spotlight result.
#[derive(Debug, Clone)]
pub struct MetadataItem {
    /// Full path to the file.
    pub path: PathBuf,
    /// Display name.
    pub display_name: String,
    /// File size in bytes.
    pub size: Option<u64>,
    /// Content type (UTI).
    pub content_type: Option<String>,
    /// Last modified time.
    pub modification_date: Option<SystemTime>,
    /// Creation time.
    pub creation_date: Option<SystemTime>,
    /// Whether this is a directory.
    pub is_directory: bool,
}

impl MetadataItem {
    /// Extracts metadata from an NSMetadataItem (as AnyObject).
    pub(crate) fn from_objc(item: &AnyObject) -> Option<Self> {
        // Extract path attribute
        let path_str = Self::get_string_attribute(item, attributes::PATH)?;
        let path = PathBuf::from(path_str);

        let display_name = Self::get_string_attribute(item, attributes::DISPLAY_NAME)
            .unwrap_or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            });

        let content_type = Self::get_string_attribute(item, attributes::CONTENT_TYPE);
        let is_directory = content_type.as_deref() == Some("public.folder");

        let size = Self::get_number_attribute(item, attributes::FS_SIZE);
        let modification_date = Self::get_date_attribute(item, attributes::CONTENT_MODIFICATION_DATE);
        let creation_date = Self::get_date_attribute(item, attributes::CONTENT_CREATION_DATE);

        Some(Self {
            path,
            display_name,
            size,
            content_type,
            modification_date,
            creation_date,
            is_directory,
        })
    }

    fn get_string_attribute(item: &AnyObject, key: &str) -> Option<String> {
        let key_str = NSString::from_str(key);
        unsafe {
            let value: Option<&NSString> = msg_send![item, valueForAttribute: &*key_str];
            value.map(|s| s.to_string())
        }
    }

    fn get_number_attribute(item: &AnyObject, key: &str) -> Option<u64> {
        let key_str = NSString::from_str(key);
        unsafe {
            let value: Option<&objc2_foundation::NSNumber> = msg_send![item, valueForAttribute: &*key_str];
            value.map(|n| n.unsignedLongLongValue())
        }
    }

    fn get_date_attribute(item: &AnyObject, key: &str) -> Option<SystemTime> {
        let key_str = NSString::from_str(key);
        unsafe {
            let value: Option<&objc2_foundation::NSDate> = msg_send![item, valueForAttribute: &*key_str];
            value.map(|d| {
                let interval = d.timeIntervalSince1970();
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs_f64(interval)
            })
        }
    }
}

/// Converts MetadataItem to the existing FileResult type for compatibility.
impl From<MetadataItem> for FileResult {
    fn from(item: MetadataItem) -> Self {
        let kind = if item.is_directory {
            FileKind::Folder
        } else {
            FileKind::from_path(&item.path)
        };

        FileResult {
            path: item.path,
            name: item.display_name,
            kind,
            size: item.size,
            modified: item.modification_date,
        }
    }
}
```

---

## 4. Task Breakdown

### Phase 1: Foundation (Week 1)

| Task | Description | Hours | Dependencies |
|------|-------------|-------|--------------|
| 1.1 | Add objc2 dependencies to Cargo.toml | 0.5h | None |
| 1.2 | Create spotlight module structure | 1h | 1.1 |
| 1.3 | Implement PredicateBuilder basics | 3h | 1.2 |
| 1.4 | Write predicate unit tests | 2h | 1.3 |
| 1.5 | Implement MetadataItem extraction | 3h | 1.2 |
| 1.6 | Write metadata extraction tests | 2h | 1.5 |

**Phase 1 Total: 11.5 hours**

### Phase 2: Query Wrapper (Week 2)

| Task | Description | Hours | Dependencies |
|------|-------------|-------|--------------|
| 2.1 | Implement MetadataQueryWrapper::new() | 2h | 1.2 |
| 2.2 | Implement set_predicate/set_search_scopes | 2h | 2.1, 1.3 |
| 2.3 | Implement notification observer with block2 | 4h | 2.2 |
| 2.4 | Implement execute_sync() | 3h | 2.3 |
| 2.5 | Implement execute_async() with tokio | 4h | 2.4 |
| 2.6 | Write query wrapper integration tests | 3h | 2.5 |

**Phase 2 Total: 18 hours**

### Phase 3: Service Layer (Week 2-3)

| Task | Description | Hours | Dependencies |
|------|-------------|-------|--------------|
| 3.1 | Implement SpotlightSearchService | 3h | 2.5 |
| 3.2 | Integrate FileQuery → Predicate conversion | 2h | 1.3, 3.1 |
| 3.3 | Implement result caching | 2h | 3.1 |
| 3.4 | Add debouncing for rapid queries | 1h | 3.1 |
| 3.5 | Write service integration tests | 3h | 3.1-3.4 |

**Phase 3 Total: 11 hours**

### Phase 4: UI Integration (Week 3)

| Task | Description | Hours | Dependencies |
|------|-------------|-------|--------------|
| 4.1 | Update file_search_view.rs to use new service | 3h | 3.5 |
| 4.2 | Update launcher.rs file search scheduling | 2h | 4.1 |
| 4.3 | Remove/deprecate mdfind usage | 1h | 4.2 |
| 4.4 | Update file_search_helper.rs | 2h | 4.2 |
| 4.5 | End-to-end testing | 3h | 4.1-4.4 |

**Phase 4 Total: 11 hours**

### Phase 5: Fallback & Polish (Week 4)

| Task | Description | Hours | Dependencies |
|------|-------------|-------|--------------|
| 5.1 | Keep mdfind as configurable fallback | 2h | 4.3 |
| 5.2 | Add error recovery and retry logic | 2h | 5.1 |
| 5.3 | Performance profiling and optimization | 4h | 4.5 |
| 5.4 | Documentation | 2h | All |
| 5.5 | Final testing and edge cases | 3h | 5.3 |

**Phase 5 Total: 13 hours**

---

## 5. Dependencies

### 5.1 Cargo.toml Additions

```toml
[target.'cfg(target_os = "macos")'.dependencies]
# Core objc2 runtime
objc2 = { version = "0.6", features = ["std"] }

# Foundation framework bindings
objc2-foundation = { version = "0.3", features = [
    "NSArray",
    "NSDictionary",
    "NSString",
    "NSURL",
    "NSDate",
    "NSNumber",
    "NSPredicate",
    "NSNotification",
    "NSNotificationCenter",
    "NSMetadataQuery",
    "NSMetadataItem",
    "NSOperationQueue",
    "block2",
] }

# Block support for callbacks
block2 = "0.5"
```

### 5.2 Feature Flags

```toml
[features]
default = []
native-spotlight = []  # Enable native objc2 Spotlight (vs mdfind fallback)
```

---

## 6. Risk Assessment

### 6.1 Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| objc2 API instability | Low | Medium | Pin to specific versions, test thoroughly |
| Thread safety issues | Medium | High | Use proper Send/Sync bounds, test on multiple threads |
| RunLoop integration | Medium | Medium | Use NSOperationQueue for async, avoid CFRunLoop |
| Block callback complexity | Medium | Medium | Extensive testing, use block2 patterns from examples |
| Memory leaks from objc | Low | Medium | Proper Retained<T> usage, test with instruments |

### 6.2 Performance Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Slower than mdfind | Low | Low | Benchmark, keep mdfind fallback |
| NSMetadataQuery startup lag | Medium | Low | Pre-warm queries, cache results |
| Large result set handling | Low | Medium | Limit results, pagination |

### 6.3 Compatibility Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| macOS version differences | Low | Medium | Test on 12, 13, 14, 15 |
| Sandbox restrictions | Medium | Low | Test with/without sandbox |
| Full Disk Access requirement | Known | Low | Prompt user, document requirement |

---

## 7. Testing Strategy

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate_builder_name_contains() {
        let builder = PredicateBuilder::new()
            .name_contains("report");
        let predicate = builder.build().unwrap();
        // Verify predicate format
    }

    #[test]
    fn test_predicate_builder_escapes_special_chars() {
        let builder = PredicateBuilder::new()
            .name_contains("test*file");
        let predicate = builder.build().unwrap();
        // Should escape * to \*
    }

    #[test]
    fn test_file_query_to_predicate_simple() {
        let query = FileQuery::parse("report");
        let predicate = file_query_to_predicate(&query).unwrap();
        // Verify conversion
    }

    #[test]
    fn test_file_query_to_predicate_with_extension() {
        let query = FileQuery::parse(".pdf budget");
        let predicate = file_query_to_predicate(&query).unwrap();
        // Should include extension filter
    }
}
```

### 7.2 Integration Tests

```rust
#[cfg(test)]
#[cfg(target_os = "macos")]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_spotlight_search_finds_home_folder() {
        let service = SpotlightSearchService::new();
        let results = service.search_files("Desktop").await.unwrap();
        
        // Should find at least the Desktop folder
        assert!(results.iter().any(|r| r.path.ends_with("Desktop")));
    }

    #[tokio::test]
    async fn test_spotlight_search_with_extension_filter() {
        let service = SpotlightSearchService::new();
        let query = FileQuery::parse(".txt");
        let results = service.search_with_query(&query).await.unwrap();
        
        // All results should be .txt files
        assert!(results.iter().all(|r| {
            r.path.extension().map_or(false, |e| e == "txt")
        }));
    }

    #[test]
    fn test_spotlight_search_sync() {
        let service = SpotlightSearchService::new();
        let results = service.search_files_sync("Documents").unwrap();
        
        assert!(!results.is_empty());
    }
}
```

### 7.3 Performance Benchmarks

```rust
#[bench]
fn bench_spotlight_search_native(b: &mut Bencher) {
    let service = SpotlightSearchService::new();
    b.iter(|| {
        let _ = service.search_files_sync("report");
    });
}

#[bench]
fn bench_spotlight_search_mdfind(b: &mut Bencher) {
    let provider = SpotlightProvider::new();  // Old mdfind-based
    b.iter(|| {
        let _ = provider.search_sync("report");
    });
}
```

---

## 8. Success Criteria

1. **Functional Parity** - All existing file search functionality preserved
2. **Performance** - Query execution ≤ mdfind baseline (within 10%)
3. **Rich Metadata** - File size, dates available without additional calls
4. **Test Coverage** - >80% coverage on new modules
5. **Error Handling** - Graceful degradation, clear error messages
6. **Documentation** - All public APIs documented with examples

---

## 9. Open Questions

1. **Live Query Support** - Should we implement `NSMetadataQueryDidUpdateNotification` for real-time updates? (Nice-to-have, defer to v2)

2. **CSSearchQuery** - Should we add semantic search support via Core Spotlight? (Requires indexing app content, likely out of scope)

3. **Sandboxing** - How should we handle sandbox entitlements for search scope access?

4. **Result Sorting** - Should sorting happen in Spotlight (via NSSortDescriptor) or in Rust post-fetch?

---

## 10. References

- [objc2 Documentation](https://docs.rs/objc2)
- [objc2-foundation Documentation](https://docs.rs/objc2-foundation)
- [Apple NSMetadataQuery Reference](https://developer.apple.com/documentation/foundation/nsmetadataquery)
- [Spotlight Query Format](https://developer.apple.com/library/archive/documentation/Carbon/Conceptual/SpotlightQuery/Concepts/QueryFormat.html)
- [File Metadata Attributes](https://developer.apple.com/documentation/coreservices/file_metadata/mditem/common_metadata_attribute_keys)

---

*Specification created: 2026-01-22*
