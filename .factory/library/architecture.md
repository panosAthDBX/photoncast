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
- **Quicklinks gotcha**: `handle_manage_quicklinks()` / `handle_browse_quicklink_library()` also flow through `open_manage_quicklinks_window()` in `crates/photoncast/src/main.rs`; auditing only `event_loop.rs` is not enough because a helper-level `runtime.block_on(storage.load_all())` there still blocks first-open window creation.
- **Launcher hide gotcha**: `LauncherWindow::hide()` is mode-sensitive. In `SearchMode::Calendar` it calls `exit_calendar_mode(cx)` and returns instead of dismissing the window, so action handlers that truly need to close the launcher cannot assume `self.hide(cx)` will behave the same in calendar and normal modes.

## Data Flow

1. User keystroke → `on_query_change()` → debounce (50ms) → async search dispatch
2. `SearchEngine::search()` → parallel provider calls via `tokio::spawn_blocking`
3. Results merged, ranked (nucleo score + frecency × 35.0 multiplier), top-K selected
4. UI re-renders with results via GPUI reactive state

## Spotlight Prefetch Status Notes

- `PrefetchStatus::Failed` is not a normal "index missing" outcome in the current implementation.
- `trigger()` maps `run_prefetch_queries() == false` to `PrefetchStatus::Failed`, but `run_prefetch_queries()` currently returns `false` only when the cancellation token is cancelled; otherwise it returns `true` even if individual query work had best-effort errors.

## Launcher Test Coverage Notes

- Active quick-select coverage in `tests/integration/keyboard_test.rs` is limited to index/bounds math.
- The natural end-to-end GPUI quick-select flow (`test_cmd_number_quick_select`) exists but is `#[ignore]` because it requires a full GPUI/Xcode environment, so binding regressions in `QuickSelectN -> quick_select(N-1)` can slip through without an actively running integration test.
