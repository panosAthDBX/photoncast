# Error Handling in Rust

## Overview

Rust's error handling is explicit and type-safe. Use `Result<T, E>` for recoverable errors and reserve panics for truly unrecoverable situations.

## When to Apply

- All functions that can fail
- Library code (especially strict)
- Application code
- Error messages and user feedback

## Core Principles

1. **Use `Result`, not panics** - Panics are for bugs, not expected failures
2. **Errors should be informative** - Include context about what went wrong
3. **Use `thiserror` for libraries** - Define clear error types
4. **Use `anyhow` for applications** - Convenient error chaining
5. **Propagate with `?`** - Clean error propagation

## ✅ DO

### DO: Define Custom Error Types with thiserror

**✅ DO**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("search index not ready")]
    IndexNotReady,
    
    #[error("invalid query: {reason}")]
    InvalidQuery { reason: String },
    
    #[error("provider '{provider}' failed: {source}")]
    ProviderError {
        provider: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    
    #[error("timeout after {duration:?}")]
    Timeout { duration: std::time::Duration },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
}
```

### DO: Use anyhow for Application Code

**✅ DO**:
```rust
use anyhow::{Context, Result, bail, ensure};

fn load_config() -> Result<Config> {
    let path = get_config_path()?;
    
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;
    
    let config: Config = toml::from_str(&content)
        .context("failed to parse config file")?;
    
    // Use ensure! for preconditions
    ensure!(config.max_results > 0, "max_results must be positive");
    
    // Use bail! for early returns
    if config.providers.is_empty() {
        bail!("at least one provider must be configured");
    }
    
    Ok(config)
}
```

### DO: Provide Context with Error Chains

**✅ DO**:
```rust
use anyhow::{Context, Result};

async fn index_applications() -> Result<()> {
    let apps_dir = get_applications_dir()
        .context("failed to locate Applications directory")?;
    
    for entry in std::fs::read_dir(&apps_dir)
        .with_context(|| format!("failed to read directory: {}", apps_dir.display()))?
    {
        let entry = entry.context("failed to read directory entry")?;
        
        index_app(&entry.path())
            .await
            .with_context(|| format!("failed to index app: {}", entry.path().display()))?;
    }
    
    Ok(())
}

// Error output:
// Error: failed to index app: /Applications/Example.app
// 
// Caused by:
//     0: failed to read Info.plist
//     1: No such file or directory (os error 2)
```

### DO: Use Result Combinators

**✅ DO**:
```rust
fn find_app(name: &str) -> Option<Application> {
    apps.iter()
        .find(|app| app.name == name)
        .cloned()
}

fn get_app_icon(app: &Application) -> Result<Icon, IconError> {
    app.icon_path
        .as_ref()
        .ok_or(IconError::NoIconPath)?
        .pipe(load_icon)
}

// Map errors
fn parse_score(s: &str) -> Result<Score, ParseError> {
    s.parse::<u32>()
        .map(Score)
        .map_err(|e| ParseError::InvalidScore(e.to_string()))
}
```

### DO: Handle Multiple Error Types

**✅ DO**:
```rust
// Option 1: Box dyn Error (simple but loses type info)
fn fallible() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = load_config()?;  // ConfigError
    let conn = connect_db()?;     // DatabaseError
    Ok(())
}

// Option 2: Custom enum (better for libraries)
#[derive(Error, Debug)]
enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    
    #[error(transparent)]
    Database(#[from] DatabaseError),
    
    #[error(transparent)]
    Search(#[from] SearchError),
}

// Option 3: anyhow (best for applications)
use anyhow::Result;

fn fallible() -> Result<()> {
    let config = load_config().context("config error")?;
    let conn = connect_db().context("database error")?;
    Ok(())
}
```

### DO: Log Errors at Appropriate Levels

**✅ DO**:
```rust
use tracing::{error, warn, info, debug};

fn process_item(item: &Item) -> Result<()> {
    match try_process(item) {
        Ok(result) => {
            debug!("processed item {}: {:?}", item.id, result);
            Ok(())
        }
        Err(e) if e.is_transient() => {
            warn!("transient error processing {}, will retry: {}", item.id, e);
            Err(e)
        }
        Err(e) => {
            error!("failed to process {}: {:?}", item.id, e);
            Err(e)
        }
    }
}
```

### DO: Use Custom Error Types for Public APIs

**✅ DO**:
```rust
// Public module exposes clear error type
pub mod search {
    use thiserror::Error;
    
    #[derive(Error, Debug)]
    #[non_exhaustive]  // Allow adding variants without breaking changes
    pub enum Error {
        #[error("query is empty")]
        EmptyQuery,
        
        #[error("index not initialized")]
        NotInitialized,
        
        #[error("search timed out")]
        Timeout,
    }
    
    pub type Result<T> = std::result::Result<T, Error>;
    
    pub fn search(query: &str) -> Result<Vec<SearchResult>> {
        if query.is_empty() {
            return Err(Error::EmptyQuery);
        }
        // ...
    }
}
```

## ❌ DON'T

### DON'T: Use unwrap() or expect() for Expected Failures

**❌ DON'T**:
```rust
fn get_app(name: &str) -> Application {
    apps.iter()
        .find(|a| a.name == name)
        .unwrap()  // Panics if not found!
}

fn load_config() -> Config {
    let content = std::fs::read_to_string("config.toml")
        .expect("config file must exist");  // Panics!
    toml::from_str(&content).expect("invalid config")
}
```
**Why**: These are expected failure modes, not bugs.

**✅ DO**:
```rust
fn get_app(name: &str) -> Option<Application> {
    apps.iter().find(|a| a.name == name).cloned()
}

fn load_config() -> Result<Config> {
    let content = std::fs::read_to_string("config.toml")?;
    let config = toml::from_str(&content)?;
    Ok(config)
}
```

### DON'T: Silently Ignore Errors

**❌ DON'T**:
```rust
fn save_state() {
    let _ = std::fs::write("state.json", data);  // Error ignored!
}

fn process_items(items: Vec<Item>) {
    for item in items {
        if let Ok(result) = process(item) {  // Errors silently dropped
            results.push(result);
        }
    }
}
```

**✅ DO**:
```rust
fn save_state() -> Result<()> {
    std::fs::write("state.json", data)?;
    Ok(())
}

fn process_items(items: Vec<Item>) -> Result<Vec<ProcessedItem>> {
    items.into_iter()
        .map(process)
        .collect()  // Propagates first error
}

// Or collect errors separately
fn process_items(items: Vec<Item>) -> (Vec<ProcessedItem>, Vec<ProcessError>) {
    let (successes, failures): (Vec<_>, Vec<_>) = items
        .into_iter()
        .map(process)
        .partition_result();
    (successes, failures)
}
```

### DON'T: Use String as Error Type

**❌ DON'T**:
```rust
fn parse_config(s: &str) -> Result<Config, String> {
    if s.is_empty() {
        return Err("config is empty".to_string());
    }
    // ...
}
```
**Why**: Loses type information, can't match on error variants.

**✅ DO**:
```rust
#[derive(Error, Debug)]
enum ConfigError {
    #[error("config is empty")]
    Empty,
    
    #[error("invalid format: {0}")]
    InvalidFormat(String),
}

fn parse_config(s: &str) -> Result<Config, ConfigError> {
    if s.is_empty() {
        return Err(ConfigError::Empty);
    }
    // ...
}
```

### DON'T: Panic in Library Code

**❌ DON'T**:
```rust
// In a library
pub fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("division by zero");  // Crashes the user's application!
    }
    a / b
}
```

**✅ DO**:
```rust
pub fn divide(a: i32, b: i32) -> Result<i32, DivisionError> {
    if b == 0 {
        return Err(DivisionError::DivideByZero);
    }
    Ok(a / b)
}
```

### DON'T: Create Errors Without Context

**❌ DON'T**:
```rust
fn load_app(path: &Path) -> Result<App> {
    let content = std::fs::read_to_string(path)?;  // Which file failed?
    let plist: Plist = plist::from_str(&content)?; // What was wrong?
    Ok(App::from_plist(plist))
}
```

**✅ DO**:
```rust
fn load_app(path: &Path) -> Result<App> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    
    let plist: Plist = plist::from_str(&content)
        .with_context(|| format!("failed to parse plist for {}", path.display()))?;
    
    Ok(App::from_plist(plist))
}
```

## Patterns

### Pattern: Fallible Initialization

```rust
pub struct SearchIndex {
    // ...
}

impl SearchIndex {
    /// Creates a new search index.
    ///
    /// # Errors
    ///
    /// Returns error if the database cannot be initialized.
    pub fn new(config: &Config) -> Result<Self, IndexError> {
        let db = Database::open(&config.db_path)?;
        let cache = Cache::new(config.cache_size)?;
        
        Ok(Self { db, cache })
    }
}
```

### Pattern: Recoverable vs Fatal Errors

```rust
enum ProcessError {
    // Recoverable - can retry or skip
    Transient(TransientError),
    
    // Fatal - must abort
    Fatal(FatalError),
}

impl ProcessError {
    fn is_recoverable(&self) -> bool {
        matches!(self, Self::Transient(_))
    }
}

fn process_with_retry(item: &Item) -> Result<Output> {
    for attempt in 1..=3 {
        match process(item) {
            Ok(output) => return Ok(output),
            Err(e) if e.is_recoverable() => {
                warn!("attempt {} failed, retrying: {}", attempt, e);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    bail!("all retry attempts failed")
}
```

### Pattern: Error Conversion Layer

```rust
// Internal module uses detailed errors
mod internal {
    #[derive(Error, Debug)]
    pub enum InternalError {
        #[error("database: {0}")]
        Db(#[from] rusqlite::Error),
        
        #[error("io: {0}")]
        Io(#[from] std::io::Error),
        
        #[error("parse: {0}")]
        Parse(#[from] serde_json::Error),
    }
}

// Public API has simpler errors
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("search failed: {message}")]
    Failed { message: String },
    
    #[error("not found")]
    NotFound,
}

impl From<internal::InternalError> for SearchError {
    fn from(e: internal::InternalError) -> Self {
        SearchError::Failed {
            message: e.to_string(),
        }
    }
}
```

## When to Panic

Panics are appropriate for:

1. **Programming errors** (bugs in your code)
   ```rust
   fn get_item(index: usize) -> &Item {
       // If index is out of bounds, it's a bug in the caller
       &self.items[index]
   }
   ```

2. **Unrecoverable state** (invariant violations)
   ```rust
   fn process(&mut self) {
       assert!(self.is_initialized, "must call init() first");
   }
   ```

3. **Test code**
   ```rust
   #[test]
   fn test_search() {
       let result = search("test").unwrap();  // OK in tests
       assert_eq!(result.len(), 5);
   }
   ```

4. **Example/demo code**
   ```rust
   fn main() {
       // OK for examples to use unwrap for brevity
       let app = App::new().unwrap();
   }
   ```

## Resources

- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [thiserror](https://docs.rs/thiserror/latest/thiserror/)
- [anyhow](https://docs.rs/anyhow/latest/anyhow/)
- [Error Handling in Rust (blog)](https://nick.groenen.me/posts/rust-error-handling/)
