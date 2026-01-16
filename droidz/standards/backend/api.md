# Search Engine & Core APIs

## Overview

PhotonCast's core is a high-performance search engine that indexes applications, files, and other system resources. This document covers the design patterns for the search infrastructure.

## When to Apply

- Implementing search providers
- Building the indexing system
- Creating extension APIs
- Designing the plugin architecture

## Core Principles

1. **Async by default** - All I/O operations are non-blocking
2. **Incremental updates** - Index changes, don't rebuild
3. **Provider-based** - Pluggable search providers
4. **Lazy evaluation** - Compute only what's needed
5. **Memory efficient** - Stream results, don't buffer everything

## ✅ DO

### DO: Use Traits for Provider Abstraction

**✅ DO**:
```rust
use async_trait::async_trait;

/// A search provider that can be queried for results
#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Unique identifier for this provider
    fn id(&self) -> &str;
    
    /// Display name for the provider
    fn name(&self) -> &str;
    
    /// Search for items matching the query
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    
    /// Check if this provider can handle a given query prefix
    fn handles_prefix(&self, prefix: &str) -> bool {
        false  // Default: no prefix handling
    }
    
    /// Priority for result ordering (higher = more important)
    fn priority(&self) -> u32 {
        100  // Default priority
    }
}

// Implementation
pub struct ApplicationProvider {
    index: Arc<ApplicationIndex>,
}

#[async_trait]
impl SearchProvider for ApplicationProvider {
    fn id(&self) -> &str { "applications" }
    fn name(&self) -> &str { "Applications" }
    fn priority(&self) -> u32 { 200 }  // Apps shown first
    
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.index.search(query, limit).await
    }
}
```

### DO: Implement Streaming Results

**✅ DO**:
```rust
use tokio::sync::mpsc;

pub struct SearchEngine {
    providers: Vec<Box<dyn SearchProvider>>,
}

impl SearchEngine {
    /// Search all providers concurrently, streaming results as they arrive
    pub fn search_stream(
        &self,
        query: &str,
        limit: usize,
    ) -> mpsc::Receiver<SearchResult> {
        let (tx, rx) = mpsc::channel(100);
        let query = query.to_string();
        let providers = self.providers.clone();
        
        tokio::spawn(async move {
            let mut handles = Vec::new();
            
            for provider in providers {
                let tx = tx.clone();
                let query = query.clone();
                
                handles.push(tokio::spawn(async move {
                    if let Ok(results) = provider.search(&query, limit).await {
                        for result in results {
                            if tx.send(result).await.is_err() {
                                break;  // Receiver dropped
                            }
                        }
                    }
                }));
            }
            
            // Wait for all providers
            for handle in handles {
                let _ = handle.await;
            }
        });
        
        rx
    }
}
```

### DO: Use Builder Pattern for Configuration

**✅ DO**:
```rust
#[derive(Clone)]
pub struct SearchConfig {
    pub max_results: usize,
    pub fuzzy_threshold: f64,
    pub include_hidden: bool,
    pub providers: Vec<String>,
    pub timeout: Duration,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 50,
            fuzzy_threshold: 0.6,
            include_hidden: false,
            providers: vec!["applications".into(), "files".into()],
            timeout: Duration::from_secs(5),
        }
    }
}

impl SearchConfig {
    pub fn builder() -> SearchConfigBuilder {
        SearchConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct SearchConfigBuilder {
    config: SearchConfig,
}

impl SearchConfigBuilder {
    pub fn max_results(mut self, n: usize) -> Self {
        self.config.max_results = n;
        self
    }
    
    pub fn fuzzy_threshold(mut self, threshold: f64) -> Self {
        self.config.fuzzy_threshold = threshold.clamp(0.0, 1.0);
        self
    }
    
    pub fn include_hidden(mut self, include: bool) -> Self {
        self.config.include_hidden = include;
        self
    }
    
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.config.providers.push(provider.into());
        self
    }
    
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }
    
    pub fn build(self) -> SearchConfig {
        self.config
    }
}
```

### DO: Use Channels for Inter-Component Communication

**✅ DO**:
```rust
use tokio::sync::{mpsc, broadcast};

/// Commands that can be sent to the indexer
pub enum IndexerCommand {
    /// Re-index a specific path
    Reindex(PathBuf),
    /// Add a new item to the index
    Add(IndexItem),
    /// Remove an item from the index
    Remove(String),
    /// Clear and rebuild the entire index
    Rebuild,
    /// Shutdown the indexer
    Shutdown,
}

/// Events emitted by the indexer
#[derive(Clone)]
pub enum IndexerEvent {
    /// Indexing started
    Started,
    /// Progress update
    Progress { current: usize, total: usize },
    /// Indexing completed
    Completed { duration: Duration, count: usize },
    /// Error occurred
    Error(String),
}

pub struct Indexer {
    command_tx: mpsc::Sender<IndexerCommand>,
    event_tx: broadcast::Sender<IndexerEvent>,
}

impl Indexer {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, _) = broadcast::channel(100);
        
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            Self::run_loop(command_rx, event_tx_clone).await;
        });
        
        Self { command_tx, event_tx }
    }
    
    pub async fn reindex(&self, path: PathBuf) -> Result<()> {
        self.command_tx.send(IndexerCommand::Reindex(path)).await?;
        Ok(())
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<IndexerEvent> {
        self.event_tx.subscribe()
    }
    
    async fn run_loop(
        mut commands: mpsc::Receiver<IndexerCommand>,
        events: broadcast::Sender<IndexerEvent>,
    ) {
        while let Some(cmd) = commands.recv().await {
            match cmd {
                IndexerCommand::Reindex(path) => {
                    let _ = events.send(IndexerEvent::Started);
                    // ... indexing logic
                    let _ = events.send(IndexerEvent::Completed { 
                        duration: Duration::from_secs(1),
                        count: 100,
                    });
                }
                IndexerCommand::Shutdown => break,
                // ... other commands
            }
        }
    }
}
```

### DO: Implement Caching with Invalidation

**✅ DO**:
```rust
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CachedSearchResult {
    pub results: Vec<SearchResult>,
    pub timestamp: Instant,
}

pub struct SearchCache {
    cache: RwLock<HashMap<String, CachedSearchResult>>,
    ttl: Duration,
    max_entries: usize,
}

impl SearchCache {
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl,
            max_entries,
        }
    }
    
    pub fn get(&self, query: &str) -> Option<Vec<SearchResult>> {
        let cache = self.cache.read();
        cache.get(query).and_then(|entry| {
            if entry.timestamp.elapsed() < self.ttl {
                Some(entry.results.clone())
            } else {
                None  // Expired
            }
        })
    }
    
    pub fn insert(&self, query: String, results: Vec<SearchResult>) {
        let mut cache = self.cache.write();
        
        // Evict old entries if at capacity
        if cache.len() >= self.max_entries {
            self.evict_oldest(&mut cache);
        }
        
        cache.insert(query, CachedSearchResult {
            results,
            timestamp: Instant::now(),
        });
    }
    
    pub fn invalidate(&self, pattern: Option<&str>) {
        let mut cache = self.cache.write();
        
        if let Some(pattern) = pattern {
            cache.retain(|k, _| !k.contains(pattern));
        } else {
            cache.clear();
        }
    }
    
    fn evict_oldest(&self, cache: &mut HashMap<String, CachedSearchResult>) {
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, v)| v.timestamp)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&oldest_key);
        }
    }
}
```

### DO: Design for Extension Points

**✅ DO**:
```rust
/// Extension API for third-party providers
pub trait Extension: Send + Sync {
    /// Extension metadata
    fn manifest(&self) -> ExtensionManifest;
    
    /// Called when extension is loaded
    fn activate(&self, ctx: &ExtensionContext) -> Result<()>;
    
    /// Called when extension is unloaded
    fn deactivate(&self) -> Result<()>;
    
    /// Get the search provider for this extension
    fn provider(&self) -> Option<Box<dyn SearchProvider>>;
    
    /// Get commands provided by this extension
    fn commands(&self) -> Vec<Command>;
}

#[derive(Debug, Clone)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

pub struct ExtensionContext {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub config: ExtensionConfig,
}
```

## ❌ DON'T

### DON'T: Block the Event Loop

**❌ DON'T**:
```rust
impl SearchProvider for FileProvider {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // DON'T: Blocking file system operations
        let entries: Vec<_> = std::fs::read_dir("/")
            .unwrap()
            .collect();  // Blocks!
        
        // DON'T: CPU-intensive work on async task
        let results: Vec<_> = entries
            .iter()
            .map(|e| expensive_scoring(e, query))  // Blocks!
            .collect();
            
        Ok(results)
    }
}
```

**✅ DO**:
```rust
impl SearchProvider for FileProvider {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // DO: Use async file operations
        let mut entries = tokio::fs::read_dir("/").await?;
        let mut results = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            results.push(entry);
        }
        
        // DO: Offload CPU work to blocking thread pool
        let query = query.to_string();
        let scored = tokio::task::spawn_blocking(move || {
            results
                .iter()
                .map(|e| expensive_scoring(e, &query))
                .collect::<Vec<_>>()
        }).await?;
        
        Ok(scored)
    }
}
```

### DON'T: Use Shared Mutable State Without Synchronization

**❌ DON'T**:
```rust
pub struct SearchIndex {
    items: Vec<IndexItem>,  // Shared mutable state!
}

impl SearchIndex {
    pub fn add(&mut self, item: IndexItem) {
        self.items.push(item);  // Race condition!
    }
}
```

**✅ DO**:
```rust
pub struct SearchIndex {
    items: Arc<RwLock<Vec<IndexItem>>>,
}

impl SearchIndex {
    pub fn add(&self, item: IndexItem) {
        let mut items = self.items.write();
        items.push(item);
    }
    
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let items = self.items.read();
        // Search logic...
    }
}
```

### DON'T: Return Large Collections When Iterators Suffice

**❌ DON'T**:
```rust
fn get_all_applications(&self) -> Vec<Application> {
    // Clones everything into memory
    self.apps.iter().cloned().collect()
}
```

**✅ DO**:
```rust
fn get_all_applications(&self) -> impl Iterator<Item = &Application> {
    self.apps.iter()
}

// Or for owned iteration
fn into_applications(self) -> impl Iterator<Item = Application> {
    self.apps.into_iter()
}
```

### DON'T: Expose Internal Types in Public API

**❌ DON'T**:
```rust
// Exposes internal implementation details
pub struct SearchEngine {
    pub cache: HashMap<String, Vec<SearchResult>>,  // Internal!
    pub providers: Vec<Box<dyn SearchProvider>>,     // Internal!
}
```

**✅ DO**:
```rust
pub struct SearchEngine {
    cache: SearchCache,
    providers: Vec<Box<dyn SearchProvider>>,
}

impl SearchEngine {
    // Public API methods only
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        // ...
    }
    
    pub fn register_provider(&mut self, provider: Box<dyn SearchProvider>) {
        // ...
    }
}
```

## Patterns

### Pattern: Result Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub kind: ResultKind,
    pub icon: Option<IconSource>,
    pub score: u32,
    pub metadata: ResultMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResultKind {
    Application { bundle_id: String, path: PathBuf },
    File { path: PathBuf, mime_type: String },
    Folder { path: PathBuf },
    Command { action: String },
    Extension { extension_id: String, data: serde_json::Value },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultMetadata {
    pub subtitle: Option<String>,
    pub keywords: Vec<String>,
    pub last_used: Option<DateTime<Utc>>,
    pub use_count: u32,
}
```

### Pattern: Priority-Based Merging

```rust
pub fn merge_results(
    provider_results: Vec<(u32, Vec<SearchResult>)>,  // (priority, results)
    limit: usize,
) -> Vec<SearchResult> {
    let mut all_results: Vec<(u32, SearchResult)> = provider_results
        .into_iter()
        .flat_map(|(priority, results)| {
            results.into_iter().map(move |r| (priority, r))
        })
        .collect();
    
    // Sort by: priority desc, then score desc
    all_results.sort_by(|a, b| {
        b.0.cmp(&a.0)  // Priority
            .then_with(|| b.1.score.cmp(&a.1.score))  // Score
    });
    
    all_results
        .into_iter()
        .map(|(_, r)| r)
        .take(limit)
        .collect()
}
```

## Resources

- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [async-trait](https://docs.rs/async-trait/latest/async_trait/)
- [Nucleo](https://docs.rs/nucleo/latest/nucleo/) - Fuzzy matching
- [Tantivy](https://docs.rs/tantivy/latest/tantivy/) - Full-text search
