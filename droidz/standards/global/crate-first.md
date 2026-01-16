# Crate-First Development Standard

## Overview

**ALWAYS search for existing crates before implementing functionality.** The Rust ecosystem has high-quality, well-tested crates for most common problems. Reinventing the wheel wastes time and introduces bugs.

## When to Apply

- Before implementing ANY non-trivial functionality
- Before writing utility functions
- Before creating data structures
- Before implementing algorithms
- Before adding platform-specific code

## Core Principle

> "If you're writing more than 50 lines of code for a common problem, you probably should be using a crate."

## ✅ DO

### DO: Search crates.io Before Implementing

**✅ DO**: Search crates.io, lib.rs, or use `cargo search`
```bash
# Search for crates
cargo search fuzzy
cargo search "file watcher"

# Or visit
# - https://crates.io
# - https://lib.rs (better search)
# - https://blessed.rs/crates (curated recommendations)
```

### DO: Check Download Counts and Maintenance Status

**✅ DO**: Evaluate crate quality before adding
```
Checklist:
□ Recent updates (within last 6 months for active crates)
□ High download count (100k+ for common functionality)
□ GitHub stars and activity
□ Documentation quality
□ No critical security advisories (cargo audit)
□ Compatible license (MIT, Apache-2.0, BSD)
```

### DO: Use Well-Known Crates for Common Tasks

**✅ DO**: Use established crates:

| Task | Crate | NOT this |
|------|-------|----------|
| Error handling | `thiserror`, `anyhow` | Manual `impl Error` |
| Serialization | `serde` | Manual parsing |
| HTTP client | `reqwest` | Raw TCP sockets |
| CLI args | `clap` | Manual `std::env::args` |
| Logging | `tracing`, `log` | `println!` everywhere |
| Async | `tokio`, `async-std` | Manual threading |
| Fuzzy search | `nucleo`, `fuzzy-matcher` | Custom implementation |
| Date/time | `chrono`, `time` | Manual calculations |
| UUID | `uuid` | Custom ID generation |
| Regex | `regex` | Character-by-character parsing |
| JSON | `serde_json` | Manual string building |
| File paths | `camino` (UTF-8 paths) | Raw `PathBuf` manipulation |
| Directories | `directories` | Hardcoded paths |
| Parallelism | `rayon` | Manual thread spawning |
| Random | `rand` | Unsafe/weak RNG |

### DO: Document Why You Chose a Crate

**✅ DO**: Add comments explaining crate choice
```toml
[dependencies]
# nucleo: High-performance fuzzy matcher, used by Helix editor
# Chosen over fuzzy-matcher for better Unicode support and speed
nucleo = "0.5"

# rusqlite: SQLite bindings with bundled SQLite
# Chosen over sled for better query flexibility and smaller binary
rusqlite = { version = "0.31", features = ["bundled"] }
```

### DO: Pin Major Versions

**✅ DO**: Use semantic versioning properly
```toml
[dependencies]
tokio = "1"           # Good: Any 1.x compatible
serde = "1.0"         # Good: Any 1.0.x compatible
uuid = "1"            # Good: Any 1.x compatible
```

## ❌ DON'T

### DON'T: Implement Common Algorithms Yourself

**❌ DON'T**:
```rust
// DON'T implement fuzzy matching yourself
fn fuzzy_match(needle: &str, haystack: &str) -> bool {
    let mut needle_chars = needle.chars().peekable();
    for c in haystack.chars() {
        if needle_chars.peek() == Some(&c) {
            needle_chars.next();
        }
    }
    needle_chars.peek().is_none()
}
```
**Why**: Buggy, slow, doesn't handle Unicode, no scoring.

**✅ DO**:
```rust
use nucleo::Matcher;

let mut matcher = Matcher::new(nucleo::Config::DEFAULT);
let score = matcher.fuzzy_match(haystack, needle);
```

### DON'T: Write Platform-Specific Code Without Checking

**❌ DON'T**:
```rust
// DON'T manually implement macOS APIs
unsafe fn get_running_applications() -> Vec<String> {
    // Hundreds of lines of unsafe Objective-C FFI...
}
```

**✅ DO**:
```rust
// Check if there's a crate first
// cocoa, objc2, or application-specific crates often exist
use cocoa::appkit::NSRunningApplication;
```

### DON'T: Copy Code from Stack Overflow

**❌ DON'T**: Copy-paste utility functions from the internet

**✅ DO**: Find the crate that code came from and use it properly

### DON'T: Add Crates Without Evaluating

**❌ DON'T**:
```toml
# Bad: Adding random crates without evaluation
some-obscure-crate = "0.0.3"  # Last updated 5 years ago, 12 downloads
```

### DON'T: Use Multiple Crates for the Same Thing

**❌ DON'T**:
```toml
[dependencies]
log = "0.4"           # Logging facade
tracing = "0.1"       # Also logging!
simple_logger = "4"   # Yet another logger!
```

**✅ DO**: Pick one ecosystem and stick with it
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Search Strategy

### Step 1: Define the Problem
```
"I need to watch file system changes"
```

### Step 2: Search Multiple Sources
```bash
# crates.io search
cargo search "file watch"
cargo search "fs notify"

# lib.rs (better categorization)
# https://lib.rs/search?q=file+system+watch

# blessed.rs (curated)
# https://blessed.rs/crates

# Ask in Rust community
# - Reddit r/rust
# - Discord
# - URLO (users.rust-lang.org)
```

### Step 3: Evaluate Options
```
notify (35M downloads, active) ✅
  - Well maintained
  - Cross-platform
  - Good documentation
  
hotwatch (500k downloads, last update 2 years) ⚠️
  - Less active
  - But simpler API
  
custom implementation ❌
  - Only if absolutely necessary
```

### Step 4: Prototype
```rust
// Try it in a scratch project first
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;

fn main() {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(2)).unwrap();
    watcher.watch("/path", RecursiveMode::Recursive).unwrap();
    // Test it works for your use case
}
```

## When Custom Implementation IS Appropriate

Custom code is justified when:

1. **Performance critical** - After profiling shows crate is bottleneck
2. **Minimal subset** - You need 5% of a crate's functionality
3. **No crate exists** - Truly novel or domain-specific
4. **License incompatible** - Crate license doesn't work for your project
5. **Security sensitive** - You need to audit every line

Even then, consider:
- Can you contribute to an existing crate?
- Can you fork and simplify?
- Is this really unique enough to warrant custom code?

## Recommended Crate Sources

1. **[lib.rs](https://lib.rs)** - Best search and categorization
2. **[crates.io](https://crates.io)** - Official registry
3. **[blessed.rs](https://blessed.rs/crates)** - Curated recommendations
4. **[awesome-rust](https://github.com/rust-unofficial/awesome-rust)** - Community list
5. **[Are We X Yet?](https://areweideyet.com/)** - Domain-specific status pages

## Code Review Checklist

When reviewing PRs, ask:

- [ ] Is there a crate for this functionality?
- [ ] Was the crate choice documented?
- [ ] Is the crate well-maintained?
- [ ] Are we using the crate idiomatically?
- [ ] Could we replace custom code with a crate?

## Examples for PhotonCast

| Feature | Use Crate | Don't Implement |
|---------|-----------|-----------------|
| Fuzzy matching | `nucleo` | Custom fuzzy algorithm |
| File watching | `notify` | FSEvents FFI |
| Global hotkey | `global-hotkey` | Raw CGEvent handling |
| App icons | `icns` | Icon parsing |
| Plist parsing | `plist` | XML parsing |
| SQLite | `rusqlite` | File-based storage |
| Keychain | `security-framework` | Raw Security.framework FFI |
| Process listing | `sysinfo` | Manual /proc parsing |
