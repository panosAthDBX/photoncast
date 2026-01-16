# Rust Coding Style

## Overview

Consistent, idiomatic Rust code that leverages the type system, follows community conventions, and produces maintainable software.

## When to Apply

- All Rust code in the project
- Code reviews
- Refactoring existing code

## Core Principles

1. **Leverage the type system** - Make illegal states unrepresentable
2. **Explicit over implicit** - Be clear about intent
3. **Zero-cost abstractions** - Don't pay for what you don't use
4. **Fail fast** - Catch errors at compile time when possible
5. **Follow rustfmt** - Don't argue about formatting

## ✅ DO

### DO: Use rustfmt and clippy

**✅ DO**: Run formatting and linting on every commit
```bash
# Format code
cargo fmt

# Run clippy with all warnings
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery

# In CI
cargo fmt --check
cargo clippy -- -D warnings
```

### DO: Use Descriptive Type Names

**✅ DO**:
```rust
// Clear, descriptive names
struct SearchQuery {
    text: String,
    max_results: usize,
    include_hidden: bool,
}

enum SearchResultKind {
    Application,
    File,
    Folder,
    Command,
}

// Type aliases for clarity
type ApplicationId = String;
type Score = u32;
```

### DO: Use Newtypes for Type Safety

**✅ DO**:
```rust
// Newtypes prevent mixing up similar types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppBundleId(String);

impl AppBundleId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Now these can't be confused
fn launch_app(bundle_id: &AppBundleId) { /* ... */ }
fn search(query: &str) { /* ... */ }
```

### DO: Make Invalid States Unrepresentable

**✅ DO**:
```rust
// Use enums to represent valid states only
enum ConnectionState {
    Disconnected,
    Connecting { attempt: u32 },
    Connected { session: Session },
    Error { reason: String, retry_after: Duration },
}

// Instead of
struct Connection {
    is_connected: bool,      // What if both are true?
    is_connecting: bool,
    session: Option<Session>, // Can be Some when disconnected?
    error: Option<String>,
}
```

### DO: Use Builder Pattern for Complex Construction

**✅ DO**:
```rust
#[derive(Default)]
pub struct SearchOptionsBuilder {
    max_results: Option<usize>,
    include_hidden: bool,
    file_types: Vec<FileType>,
}

impl SearchOptionsBuilder {
    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = Some(n);
        self
    }
    
    pub fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }
    
    pub fn file_type(mut self, ft: FileType) -> Self {
        self.file_types.push(ft);
        self
    }
    
    pub fn build(self) -> SearchOptions {
        SearchOptions {
            max_results: self.max_results.unwrap_or(50),
            include_hidden: self.include_hidden,
            file_types: self.file_types,
        }
    }
}

// Usage
let options = SearchOptionsBuilder::default()
    .max_results(100)
    .include_hidden(true)
    .file_type(FileType::Application)
    .build();
```

### DO: Prefer Iterators Over Loops

**✅ DO**:
```rust
// Iterator chains are more expressive and often faster
let top_results: Vec<SearchResult> = results
    .iter()
    .filter(|r| r.score > threshold)
    .take(10)
    .cloned()
    .collect();

// Parallel iteration with rayon
use rayon::prelude::*;
let scores: Vec<u32> = items
    .par_iter()
    .map(|item| calculate_score(item))
    .collect();
```

### DO: Use `impl Trait` in Function Signatures

**✅ DO**:
```rust
// Accept any iterator, return opaque type
fn process_items(items: impl Iterator<Item = &str>) -> impl Iterator<Item = String> {
    items
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase())
}

// Accept anything string-like
fn search(query: impl AsRef<str>) {
    let query = query.as_ref();
    // ...
}
```

### DO: Document Public APIs

**✅ DO**:
```rust
/// Performs fuzzy search across indexed items.
///
/// # Arguments
///
/// * `query` - The search query string
/// * `options` - Search configuration options
///
/// # Returns
///
/// A vector of search results sorted by relevance score (highest first).
///
/// # Example
///
/// ```
/// let results = search("firefox", SearchOptions::default());
/// for result in results {
///     println!("{}: {}", result.name, result.score);
/// }
/// ```
///
/// # Errors
///
/// Returns `SearchError::IndexNotReady` if the search index is still building.
pub fn search(query: &str, options: SearchOptions) -> Result<Vec<SearchResult>, SearchError> {
    // ...
}
```

### DO: Use Module Organization

**✅ DO**:
```rust
// src/search/mod.rs
mod fuzzy;
mod indexer;
mod providers;

pub use fuzzy::FuzzyMatcher;
pub use indexer::SearchIndexer;
pub use providers::{AppProvider, FileProvider};

// Re-export common types at crate root
// src/lib.rs
pub mod search;
pub use search::{FuzzyMatcher, SearchIndexer};
```

## ❌ DON'T

### DON'T: Use `unwrap()` or `expect()` in Library Code

**❌ DON'T**:
```rust
fn get_config() -> Config {
    let content = std::fs::read_to_string("config.toml").unwrap(); // Panics!
    toml::from_str(&content).expect("invalid config") // Panics!
}
```
**Why**: Panics crash the application. Handle errors properly.

**✅ DO**:
```rust
fn get_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")?;
    let config = toml::from_str(&content)?;
    Ok(config)
}
```

### DON'T: Clone When You Can Borrow

**❌ DON'T**:
```rust
fn process(data: String) {  // Takes ownership unnecessarily
    println!("{}", data);
}

fn main() {
    let s = String::from("hello");
    process(s.clone());  // Unnecessary clone
    process(s);
}
```

**✅ DO**:
```rust
fn process(data: &str) {  // Borrows instead
    println!("{}", data);
}

fn main() {
    let s = String::from("hello");
    process(&s);
    process(&s);  // Can reuse
}
```

### DON'T: Use `String` When `&str` Suffices

**❌ DON'T**:
```rust
fn greet(name: String) {  // Forces allocation
    println!("Hello, {}!", name);
}

greet("World".to_string());  // Unnecessary allocation
```

**✅ DO**:
```rust
fn greet(name: &str) {
    println!("Hello, {}!", name);
}

greet("World");  // No allocation
greet(&my_string);  // Works with String too
```

### DON'T: Use Boolean Arguments

**❌ DON'T**:
```rust
fn search(query: &str, case_sensitive: bool, include_hidden: bool) {
    // What does search("foo", true, false) mean?
}
```

**✅ DO**:
```rust
#[derive(Default)]
struct SearchOptions {
    case_sensitive: bool,
    include_hidden: bool,
}

fn search(query: &str, options: SearchOptions) {
    // Clear what options are set
}

// Or use enums for binary choices
enum CaseSensitivity {
    Sensitive,
    Insensitive,
}
```

### DON'T: Use `&Vec<T>` or `&String` in Parameters

**❌ DON'T**:
```rust
fn process_items(items: &Vec<String>) { /* ... */ }
fn process_text(text: &String) { /* ... */ }
```
**Why**: More restrictive than necessary.

**✅ DO**:
```rust
fn process_items(items: &[String]) { /* ... */ }  // Accepts &Vec, &[], arrays
fn process_text(text: &str) { /* ... */ }         // Accepts &String, &str, literals
```

### DON'T: Write Long Functions

**❌ DON'T**: Functions over 50 lines are hard to understand

**✅ DO**: Extract logical units into separate functions
```rust
fn process_search_results(results: Vec<RawResult>) -> Vec<SearchResult> {
    results
        .into_iter()
        .map(normalize_result)
        .filter(is_valid_result)
        .map(enrich_with_metadata)
        .collect()
}

fn normalize_result(raw: RawResult) -> SearchResult { /* ... */ }
fn is_valid_result(result: &SearchResult) -> bool { /* ... */ }
fn enrich_with_metadata(result: SearchResult) -> SearchResult { /* ... */ }
```

### DON'T: Ignore Compiler Warnings

**❌ DON'T**:
```rust
#![allow(warnings)]  // Never do this
#![allow(dead_code)] // Only temporarily during development
```

**✅ DO**: Fix all warnings, or use targeted allows with explanations
```rust
#[allow(dead_code)] // Used in tests only
fn test_helper() { /* ... */ }
```

## Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Crates | snake_case | `my_crate` |
| Modules | snake_case | `search_engine` |
| Types (struct, enum, trait) | PascalCase | `SearchResult` |
| Functions | snake_case | `find_applications` |
| Methods | snake_case | `get_score` |
| Local variables | snake_case | `max_results` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RESULTS` |
| Statics | SCREAMING_SNAKE_CASE | `GLOBAL_CONFIG` |
| Type parameters | Single uppercase or PascalCase | `T`, `Item` |
| Lifetimes | Short lowercase | `'a`, `'ctx` |

## Common Patterns

### Conversion Traits
```rust
// From/Into for infallible conversions
impl From<RawResult> for SearchResult {
    fn from(raw: RawResult) -> Self { /* ... */ }
}

// TryFrom/TryInto for fallible conversions
impl TryFrom<&str> for AppBundleId {
    type Error = ParseError;
    fn try_from(s: &str) -> Result<Self, Self::Error> { /* ... */ }
}
```

### Default Trait
```rust
impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_results: 50,
            include_hidden: false,
            case_sensitive: false,
        }
    }
}
```

### Display for User-Facing Output
```rust
impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.kind)
    }
}
```

## Clippy Configuration

```toml
# Cargo.toml or .cargo/config.toml
[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"

# Allow specific lints with reason
module_name_repetitions = "allow"  # Sometimes clearer
too_many_lines = "allow"           # Complex functions exist
```

## Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
