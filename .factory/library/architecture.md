# Architecture — PhotonCast

## System Overview

PhotonCast is a ~101K-line pure Rust macOS launcher application built on GPUI (GPU-accelerated UI from Zed editor). It provides instant search across apps, commands, files, and extensions.

## Crate Structure

```
photoncast (binary)         — Main app: launcher UI, event loop, preferences, file search view
├── photoncast-core         — Foundation: search engine, providers, indexer, extensions, platform APIs, storage
│   ├── photoncast-theme    — Catppuccin color scheme (4 flavors + accent colors)
│   └── photoncast-extension-api — ABI-stable extension trait (abi_stable)
├── photoncast-apps         — App management: uninstaller, force quit, auto quit, sleep timer
├── photoncast-calculator   — Calculator: math parser, unit/currency/datetime conversions
├── photoncast-calendar     — Calendar: EventKit integration, conference link detection
├── photoncast-clipboard    — Clipboard: encrypted history, monitoring, search
├── photoncast-quicklinks   — Quick links: bookmarks, browser import, URL expansion
├── photoncast-timer        — Sleep timer: NLP parsing, scheduled actions
└── photoncast-window       — Window management: layouts, multi-display, accessibility API, animations
```

## Key Architectural Patterns

- **GPUI rendering**: Immediate-mode GPU-accelerated UI at 120 FPS via Metal
- **Async I/O**: Tokio runtime for non-blocking operations (file search, DB, extensions)
- **SQLite storage**: rusqlite (bundled) for persistence — each subsystem owns its DB
- **Search pipeline**: Parallel providers → merge → deduplicate → frecency-based ranking → top-K
- **Extension system**: ABI-stable dylib extensions via `abi_stable` + `libloading`
- **Error handling**: `thiserror` for library errors, `anyhow` for application errors

## Threading Model

- **Main thread**: GPUI render loop + event handling
- **Tokio runtime**: Shared `tokio::runtime::Handle` for async ops, passed via `Arc`
- **Background executor**: GPUI's `cx.background_executor()` for CPU-bound work
- **CRITICAL**: `block_on()` on the main thread freezes the UI — must use async dispatch

## Data Flow

1. User keystroke → `on_query_change()` → debounce (50ms) → async search dispatch
2. `SearchEngine::search()` → parallel provider calls via `tokio::spawn_blocking`
3. Results merged, ranked (nucleo score + frecency × 35.0 multiplier), top-K selected
4. UI re-renders with results via GPUI reactive state
