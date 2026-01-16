# PhotonCast - Technical Stack

> Comprehensive technology decisions for the PhotonCast macOS launcher

This document details the complete technical stack for PhotonCast. For the canonical reference, see [`../standards/global/tech-stack.md`](../standards/global/tech-stack.md).

## Stack Summary

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Language** | Rust 2021 (MSRV 1.75+) | Performance, safety, native code |
| **GUI** | GPUI + gpui-component | GPU-accelerated 120 FPS rendering |
| **Async** | Tokio | Non-blocking I/O, concurrency |
| **Search** | nucleo | High-performance fuzzy matching |
| **Storage** | rusqlite | Local database for history, config |
| **macOS** | objc2, cocoa, core-foundation | Native platform integration |
| **Errors** | thiserror + anyhow | Idiomatic error handling |
| **Extensions** | Node.js sidecar | Raycast extension compatibility |

---

## Core Technologies

### Language: Rust

**Version:** 2021 Edition, MSRV 1.75+

**Why Rust for a Launcher?**

1. **Predictable Performance** - No garbage collection pauses during critical UI moments
2. **Small Binary** - ~10MB optimized release vs 100MB+ Electron bundles
3. **Memory Safety** - No crashes from null pointers or buffer overflows
4. **Native Integration** - Direct access to macOS APIs via FFI
5. **Ecosystem** - Rich crates for everything we need

```toml
# Rust toolchain configuration
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-analyzer"]
```

### GUI Framework: GPUI

**Crates:** `gpui`, `gpui-component`

GPUI is the GPU-accelerated UI framework extracted from Zed editor. It provides:

- **120 FPS rendering** via Metal on macOS
- **Immediate mode API** - Simple, declarative component model
- **Built-in accessibility** - VoiceOver support
- **First-class macOS integration** - Native menus, windows, events

**Reference:** [Loungy](https://github.com/MatthiasGrandl/Loungy) demonstrates GPUI used for the exact same use case (launcher).

```rust
// Example GPUI component structure
impl Render for SearchBar {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .child(Icon::new(IconName::Search))
            .child(
                TextInput::new(self.query.clone())
                    .placeholder("Search...")
                    .on_change(cx.listener(Self::on_input_change))
            )
    }
}
```

### Async Runtime: Tokio

**Features:** `rt-multi-thread`, `macros`, `fs`, `process`, `sync`

All I/O operations are async to keep the UI thread responsive:

- File system operations (indexing, file search)
- Process spawning (launching apps, running scripts)
- SQLite operations (via async wrappers)
- Extension execution

```rust
// Async pattern for non-blocking operations
async fn index_applications(&self) -> Result<Vec<AppEntry>> {
    let apps_dir = Path::new("/Applications");
    let mut entries = Vec::new();
    
    let mut dir = tokio::fs::read_dir(apps_dir).await?;
    while let Some(entry) = dir.next_entry().await? {
        if entry.path().extension() == Some("app".as_ref()) {
            entries.push(self.parse_app_metadata(&entry.path()).await?);
        }
    }
    
    Ok(entries)
}
```

---

## Raycast Extension Runtime

PhotonCast achieves Raycast extension compatibility through a **Node.js sidecar process** that runs extensions in isolation while communicating with the main Rust application.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                PhotonCast (Rust/GPUI)                       │
│  ┌─────────────────┐    ┌──────────────────────────────┐   │
│  │  Native Search  │    │   Extension Host Manager      │   │
│  │  Providers      │    │   - Lifecycle management      │   │
│  └─────────────────┘    │   - IPC coordination          │   │
│                         │   - UI translation            │   │
│                         └──────────────┬───────────────┘   │
│                                        │ JSON-RPC/stdio    │
├────────────────────────────────────────┼────────────────────┤
│                         ┌──────────────▼───────────────┐   │
│                         │   Node.js Sidecar Process    │   │
│                         │   ┌──────────────────────┐   │   │
│                         │   │  @raycast/api shim   │   │   │
│                         │   │  - UI Components     │   │   │
│                         │   │  - Storage/Clipboard │   │   │
│                         │   │  - React renderer    │   │   │
│                         │   └──────────────────────┘   │   │
│                         │              │               │   │
│                         │   ┌──────────▼──────────┐   │   │
│                         │   │  Raycast Extension  │   │   │
│                         │   │  (TypeScript/React) │   │   │
│                         │   └─────────────────────┘   │   │
│                         └──────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Extension Runtime Components

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Sidecar Binary** | Node.js (bundled via pkg or similar) | Execute JavaScript extensions |
| **IPC Protocol** | JSON-RPC over stdio | Communication between Rust and Node.js |
| **API Shim** | TypeScript package | `@raycast/api` compatibility layer |
| **React Renderer** | Custom renderer | Translate React components to IPC messages |
| **Store Client** | HTTP/REST | Fetch extensions from Raycast Store |

### Key Dependencies (Sidecar)

```json
{
  "dependencies": {
    "react": "^18.0.0",
    "react-reconciler": "^0.29.0",
    "zod": "^3.0.0"
  }
}
```

### IPC Protocol Example

```typescript
// Host -> Sidecar
{
  "jsonrpc": "2.0",
  "method": "extension.run",
  "params": {
    "extensionId": "github",
    "command": "search-repos",
    "arguments": {}
  },
  "id": 1
}

// Sidecar -> Host (UI render)
{
  "jsonrpc": "2.0",
  "method": "ui.render",
  "params": {
    "component": "List",
    "props": { "isLoading": false },
    "children": [
      {
        "type": "ListItem",
        "props": { "title": "raycast/extensions", "subtitle": "Official extensions" }
      }
    ]
  }
}
```

### Compatibility Considerations

| Raycast Feature | Support Level | Implementation |
|-----------------|---------------|----------------|
| List, Grid, Detail, Form | ✅ Full | Translated to GPUI equivalents |
| LocalStorage | ✅ Full | SQLite-backed per-extension storage |
| Clipboard | ✅ Full | Native macOS clipboard APIs |
| showToast/showHUD | ✅ Full | Native notifications |
| OAuth | ⚠️ Partial | Basic flows, no browser popup |
| runAppleScript | ❌ None | macOS-only, throws error |
| AI APIs | ❌ None | Explicitly not supported |

See [`../standards/backend/extensions.md`](../standards/backend/extensions.md) for full implementation details.

---

## Search & Indexing

### Fuzzy Matching: nucleo

**Why nucleo?**

- Used by Helix editor for file picker (battle-tested)
- Extremely fast (handles 100k+ items)
- Unicode-aware
- Customizable scoring

```rust
use nucleo_matcher::{Matcher, Config};
use nucleo_matcher::pattern::{Pattern, CaseMatching, Normalization};

fn fuzzy_search(query: &str, items: &[String]) -> Vec<(usize, u32)> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    
    items.iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            pattern.score(item.chars(), &mut matcher)
                .map(|score| (idx, score))
        })
        .collect()
}
```

### Content Search: tantivy (Future)

For Phase 3 file content search, we'll use tantivy:

- Full-text search engine in Rust
- Handles millions of documents
- Index stored locally
- Incremental updates

---

## Data Storage

### Local Database: rusqlite

**Features:** `bundled` (statically linked SQLite)

Used for:
- Clipboard history
- Usage statistics (for ranking)
- Extension data
- User preferences

```rust
// Schema example
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS clipboard_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_type TEXT NOT NULL,
    content BLOB NOT NULL,
    preview TEXT,
    created_at INTEGER NOT NULL,
    app_bundle_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_clipboard_created_at 
    ON clipboard_history(created_at DESC);

CREATE TABLE IF NOT EXISTS app_usage (
    bundle_id TEXT PRIMARY KEY,
    launch_count INTEGER DEFAULT 0,
    last_launched INTEGER
);
"#;
```

### Configuration: TOML

User configuration stored in `~/.config/photoncast/config.toml`:

```toml
[general]
hotkey = "cmd+space"
theme = "dark"
max_results = 10

[clipboard]
enabled = true
max_history = 1000

[extensions]
auto_update = true
```

---

## macOS Platform Integration

### Core Bindings

| Crate | Purpose |
|-------|---------|
| `objc2` | Safe Objective-C runtime bindings |
| `cocoa` | Cocoa framework (NSWorkspace, NSApplication) |
| `core-foundation` | Core Foundation types (CFString, CFURL) |
| `accessibility-sys` | Accessibility APIs for global hotkey |

### Key Integrations

#### Global Hotkey
```rust
// Register global hotkey via Carbon Events or Accessibility APIs
use core_foundation::runloop::CFRunLoop;

fn register_hotkey(key_code: u32, modifiers: u32, callback: impl Fn()) {
    // Implementation using EventHotKeyRef
}
```

#### Application Launching
```rust
use cocoa::appkit::NSWorkspace;

fn launch_app(bundle_id: &str) -> Result<()> {
    let workspace = NSWorkspace::sharedWorkspace();
    // Use launchApplication or openURL
}
```

#### Spotlight Integration
```rust
// Query Spotlight for files
use core_foundation::base::TCFType;

async fn spotlight_search(query: &str) -> Vec<FileResult> {
    // NSMetadataQuery implementation
}
```

---

## Error Handling

### Library Errors: thiserror

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Failed to index applications: {0}")]
    IndexError(#[from] std::io::Error),
    
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
}
```

### Application Errors: anyhow

```rust
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let path = config_path()?;
    let content = std::fs::read_to_string(&path)
        .context("Failed to read config file")?;
    toml::from_str(&content)
        .context("Invalid config format")
}
```

---

## Build & Distribution

### Release Profile

```toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for better optimization
strip = true            # Strip symbols
panic = "abort"         # Smaller binary, no unwinding
opt-level = 3           # Maximum optimization
```

### macOS Bundle

PhotonCast is distributed as a `.app` bundle:

```
PhotonCast.app/
├── Contents/
│   ├── Info.plist
│   ├── MacOS/
│   │   └── PhotonCast    # Binary
│   ├── Resources/
│   │   ├── AppIcon.icns
│   │   └── assets/
│   └── Frameworks/       # If needed
```

**Info.plist Essentials:**
```xml
<key>LSUIElement</key>
<true/>                     <!-- Hide from dock -->
<key>NSHighResolutionCapable</key>
<true/>                     <!-- Retina support -->
<key>NSAppleEventsUsageDescription</key>
<string>PhotonCast needs accessibility access for global hotkey</string>
```

---

## Development Tools

### Required Tools

| Tool | Purpose |
|------|---------|
| `rustfmt` | Code formatting |
| `clippy` | Linting |
| `rust-analyzer` | IDE support |
| `cargo-watch` | Auto-rebuild on changes |
| `cargo-nextest` | Faster test runner |

### CI Pipeline

```yaml
# GitHub Actions
jobs:
  build:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo build --release
```

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Cold start | < 100ms | Time to first frame |
| Hotkey response | < 50ms | Key press to window visible |
| Search latency | < 30ms | Keystroke to results |
| Memory (idle) | < 50MB | Activity Monitor |
| Memory (active) | < 100MB | With large clipboard history |
| Binary size | < 15MB | Release build |
| FPS | 120 | Consistent during animations |

---

## Security Considerations

1. **No network requests** - Core app never phones home
2. **Sandboxed extensions** - Extensions can't access arbitrary system
3. **No telemetry** - Zero data collection by default
4. **Secure storage** - Sensitive data encrypted at rest
5. **Code signing** - Notarized for Gatekeeper

---

## References

- [GPUI Documentation](https://gpui.rs)
- [gpui-component Library](https://github.com/longbridge/gpui-component)
- [Loungy Launcher](https://github.com/MatthiasGrandl/Loungy) - Reference implementation
- [Zed Editor](https://github.com/zed-industries/zed) - GPUI in production
- [nucleo Fuzzy Matcher](https://github.com/helix-editor/nucleo)
- [Rust macOS Development Guide](https://rust-lang.github.io/rust-bindgen/)

---

*For the authoritative tech stack reference, see [`../standards/global/tech-stack.md`](../standards/global/tech-stack.md)*
