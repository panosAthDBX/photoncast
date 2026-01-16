# Tech Stack - PhotonCast

> A Rust-based macOS spotlight/launcher replacement inspired by Raycast

## Overview

PhotonCast is built as a **pure Rust** application using GPUI, the GPU-accelerated UI framework from Zed. This provides native macOS performance, 120 FPS rendering, and a snappy user experience critical for a launcher application.

## Core Stack

### Framework & Runtime
- **GUI Framework:** GPUI + gpui-component
- **Language:** Rust (Edition 2021, MSRV 1.75+)
- **Async Runtime:** Tokio
- **Build System:** Cargo

### Why GPUI?
1. **Proven for launchers** - Loungy (1.6k stars) uses GPUI for the same use case
2. **GPU-accelerated** - 120 FPS rendering for buttery smooth UX
3. **Pure Rust** - No JavaScript, no web views, no Electron bloat
4. **macOS-native** - First-class macOS support with native integration
5. **Component library** - gpui-component provides 60+ ready-to-use components

## Dependencies

### GUI & Rendering
```toml
[dependencies]
gpui = "0.1"                    # Core GPUI framework
gpui-component = "0.1"          # UI component library (buttons, inputs, menus, etc.)
```

### macOS Integration
```toml
[dependencies]
objc2 = "0.5"                   # Safe Objective-C bindings
cocoa = "0.26"                  # Cocoa framework bindings
core-foundation = "0.10"        # Core Foundation types
accessibility-sys = "0.1"       # Accessibility APIs
```

### Search & Indexing
```toml
[dependencies]
nucleo = "0.5"                  # High-performance fuzzy matcher (used by Helix editor)
# OR
fuzzy-matcher = "0.3"           # Alternative fuzzy matching
tantivy = "0.22"                # Full-text search engine (if needed for file content)
```

### Async & Concurrency
```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs", "process", "sync"] }
futures = "0.3"
parking_lot = "0.12"            # Faster mutexes
```

### Data & Serialization
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"                    # Config files
```

### Storage
```toml
[dependencies]
directories = "5.0"             # Platform-specific directories
rusqlite = { version = "0.31", features = ["bundled"] }  # Local database
# OR
sled = "0.34"                   # Embedded database alternative
```

### Error Handling
```toml
[dependencies]
thiserror = "2.0"               # Library error types
anyhow = "1.0"                  # Application error handling
```

### Logging & Diagnostics
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Utilities
```toml
[dependencies]
chrono = "0.4"                  # Date/time handling
uuid = { version = "1", features = ["v4", "serde"] }
regex = "1"
once_cell = "1"                 # Lazy statics
```

### Dev Dependencies
```toml
[dev-dependencies]
criterion = "0.5"               # Benchmarking
proptest = "1"                  # Property-based testing
tempfile = "3"                  # Temporary files for tests
```

## Project Structure

```
photoncast/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library root (for testing)
│   ├── app/                    # Application state & lifecycle
│   │   ├── mod.rs
│   │   ├── state.rs
│   │   └── config.rs
│   ├── ui/                     # GPUI views and components
│   │   ├── mod.rs
│   │   ├── launcher.rs         # Main launcher window
│   │   ├── search_bar.rs       # Search input component
│   │   ├── results_list.rs     # Results display
│   │   └── components/         # Reusable UI components
│   ├── search/                 # Search engine
│   │   ├── mod.rs
│   │   ├── indexer.rs          # File/app indexing
│   │   ├── fuzzy.rs            # Fuzzy matching
│   │   └── providers/          # Search providers (apps, files, etc.)
│   ├── extensions/             # Plugin/extension system
│   │   ├── mod.rs
│   │   └── api.rs
│   ├── platform/               # macOS-specific code
│   │   ├── mod.rs
│   │   ├── hotkey.rs           # Global hotkey registration
│   │   ├── accessibility.rs    # Accessibility APIs
│   │   └── spotlight.rs        # Spotlight metadata
│   └── utils/                  # Shared utilities
│       ├── mod.rs
│       └── fs.rs
├── resources/                  # Assets
│   ├── icons/
│   └── themes/
├── tests/                      # Integration tests
│   └── integration/
└── benches/                    # Benchmarks
    └── search_bench.rs
```

## Build Configuration

### Cargo.toml Profile
```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
opt-level = 3

[profile.dev]
opt-level = 1                   # Faster dev builds with some optimization

[profile.dev.package."*"]
opt-level = 3                   # Optimize dependencies even in dev
```

### macOS Bundle (Info.plist essentials)
- `LSUIElement = true` - Hide from dock
- `NSHighResolutionCapable = true` - Retina support
- `LSBackgroundOnly = false` - Allow foreground activation
- Accessibility permissions for global hotkey

## Key Design Decisions

1. **No web views** - Pure native UI for minimal latency
2. **Async-first** - All I/O operations are async
3. **Incremental indexing** - Background file system watching
4. **Extension sandbox** - Extensions run in isolated contexts
5. **Lazy loading** - Results load incrementally as user types

## Version Requirements

- **Rust:** 1.75.0+
- **macOS:** 12.0+ (Monterey)
- **Xcode Command Line Tools:** Required for building

## References

- [GPUI Documentation](https://gpui.rs)
- [gpui-component](https://github.com/longbridge/gpui-component)
- [Loungy](https://github.com/MatthiasGrandl/Loungy) - Reference implementation
- [Zed Editor](https://github.com/zed-industries/zed) - GPUI in production
