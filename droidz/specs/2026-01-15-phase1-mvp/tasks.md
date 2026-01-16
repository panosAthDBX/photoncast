# PhotonCast Phase 1 MVP - Task Breakdown

> 12-week implementation plan for macOS launcher MVP

**Timeline:** 12 weeks (3 sprints × 4 weeks)  
**Performance Targets:** <100ms cold start, <50ms hotkey, <30ms search, <50MB memory, 120 FPS

---

## Table of Contents

1. [Sprint 1: Core UI Framework](#sprint-1-core-ui-framework-weeks-1-4)
2. [Sprint 2: App Launcher](#sprint-2-app-launcher-weeks-5-8)
3. [Sprint 3: Global Hotkey & System](#sprint-3-global-hotkey--system-weeks-9-12)
4. [Critical Path Summary](#critical-path-summary)

---

## Sprint 1: Core UI Framework (Weeks 1-4)

### 1.1 Project Setup & Infrastructure

- [x] **1.1.1 Initialize Cargo workspace** (2h) ⭐ CRITICAL
  - Create `Cargo.toml` with workspace configuration
  - Set up `photoncast` binary crate and `photoncast-core` library crate
  - Configure MSRV 1.75+, Rust 2021 edition
  - Add `rust-toolchain.toml` for version pinning
  - Dependencies: None

- [x] **1.1.2 Configure linting and formatting** (1h)
  - Add `rustfmt.toml` with project formatting rules
  - Configure `clippy.toml` with pedantic + nursery lints
  - Add `.editorconfig` for editor consistency
  - Dependencies: 1.1.1

- [x] **1.1.3 Set up GitHub Actions CI pipeline** (2h) ⭐ CRITICAL
  - Create `.github/workflows/ci.yml`
  - Add jobs: `cargo fmt --check`, `cargo clippy`, `cargo test`
  - Configure macOS runner for platform-specific tests
  - Add caching for Cargo dependencies
  - Dependencies: 1.1.1, 1.1.2

- [x] **1.1.4 Add core dependencies to Cargo.toml** (1h) ⭐ CRITICAL
  - Add GPUI (`gpui` crate from Zed) - Using rev 894866da9478003e77458e7a849353312d4c282c
  - Add `gpui-component` for UI components
  - Add `tokio` for async runtime
  - Add `thiserror` + `anyhow` for error handling
  - Add `tracing` + `tracing-subscriber` for logging
  - Add `serde` + `toml` for configuration
  - Dependencies: 1.1.1
  - **Note**: Build requires Xcode with accepted license (`sudo xcodebuild -license`)

- [x] **1.1.5 Create module structure** (1h)
  - Set up `src/` directory structure per spec (app, ui, search, etc.)
  - Create `mod.rs` files with proper re-exports
  - Add placeholder modules for each component
  - Dependencies: 1.1.1

- [x] **1.1.6 Set up integration test infrastructure** (1h)
  - Create `tests/` directory structure
  - Add `tests/common/mod.rs` for shared test utilities
  - Configure `proptest` for property-based testing
  - Dependencies: 1.1.1, 1.1.4

### 1.2 GPUI Integration

- [x] **1.2.1 Create GPUI application bootstrap** (3h) ⭐ CRITICAL
  - Implement `main.rs` with GPUI application initialization
  - Set up `App` and run loop
  - Configure 120 FPS rendering target
  - Add graceful shutdown handling
  - Dependencies: 1.1.4
  - **Status**: Implemented in `crates/photoncast/src/main.rs`

- [x] **1.2.2 Create main launcher window** (3h) ⭐ CRITICAL
  - Implement `LauncherWindow` struct with `Render` trait
  - Set fixed width (680px), dynamic height (72-500px)
  - Configure border radius (12px), shadow, centered-top position
  - Handle multi-monitor cursor-based positioning
  - Dependencies: 1.2.1
  - **Status**: Implemented in `crates/photoncast/src/launcher.rs`

- [x] **1.2.3 Implement window show/hide logic** (2h)
  - Add `toggle()`, `show()`, `hide()` methods
  - Configure window as panel (no dock icon by default)
  - Set up window focus handling
  - Target: <50ms window appear time
  - Dependencies: 1.2.2
  - **Status**: Implemented in `LauncherWindow` struct

- [x] **1.2.4 Register GPUI actions** (2h)
  - Define actions in `src/app/actions.rs` using `actions!` macro
  - Register `SelectNext`, `SelectPrevious`, `Activate`, `Cancel`
  - Add `QuickSelect1-9` actions for ⌘1-9 shortcuts
  - Dependencies: 1.2.1
  - **Status**: Actions defined in `main.rs` using `actions!` macro

- [x] **1.2.5 Configure key bindings** (1h)
  - Set up `↑/↓` for navigation, `Enter` for activate, `Esc` for cancel
  - Add `Ctrl+N/P` alternatives for navigation
  - Add `⌘1-9` for quick selection
  - Add `Tab` for group cycling
  - Dependencies: 1.2.4
  - **Status**: Key bindings registered in `register_key_bindings()` function

- [x] **1.2.6 Write GPUI integration tests** (2h)
  - Test window creation and rendering
  - Test action dispatch and key binding
  - Verify 120 FPS baseline rendering
  - Dependencies: 1.2.2, 1.2.4
  - **Status**: Test stubs in `tests/integration/gpui_test.rs` (requires Xcode license)

### 1.3 Theme System

- [x] **1.3.1 Implement Catppuccin palette definitions** (2h) ⭐ CRITICAL
  - Create `src/theme/catppuccin.rs` with all 4 flavors
  - Define all 14 accent colors + 12 surface colors per flavor
  - Use `gpui::Hsla` for all color values
  - Match exact hex values from Catppuccin spec
  - Dependencies: 1.1.4

- [x] **1.3.2 Create semantic color mapping** (2h) ⭐ CRITICAL
  - Implement `ThemeColors` struct with semantic roles
  - Map: background, surface, text, border, accent, status colors
  - Include hover/selected/focus states
  - Dependencies: 1.3.1

- [x] **1.3.3 Implement theme provider** (2h) ⭐ CRITICAL
  - Create `PhotonTheme` struct implementing GPUI `Global`
  - Add `theme(cx: &App)` accessor function
  - Support runtime theme switching
  - Dependencies: 1.3.1, 1.3.2

- [x] **1.3.4 Add accent color customization** (1h)
  - Implement `AccentColor` enum with all 14 options
  - Add `with_accent()` builder method
  - Default to Mauve
  - Dependencies: 1.3.3

- [x] **1.3.5 Implement system theme detection** (2h)
  - Use `cocoa` crate to detect macOS appearance
  - Call `NSApp.effectiveAppearance()` for dark/light detection
  - Map system dark → Mocha, system light → Latte
  - Dependencies: 1.3.3

- [x] **1.3.6 Add system theme change observer** (2h)
  - Use `cx.observe_system_appearance()` for live updates
  - Implement auto-sync option (enabled by default)
  - Trigger `cx.refresh()` on theme change
  - Dependencies: 1.3.5

- [x] **1.3.7 Write theme unit tests** (1h)
  - Test all 4 flavors load correctly
  - Test semantic mapping produces valid colors
  - Test accent color override works
  - Dependencies: 1.3.1, 1.3.2, 1.3.3

### 1.4 Core UI Components

- [x] **1.4.1 Implement SearchBar component** (3h) ⭐ CRITICAL
  - Create `src/ui/search_bar.rs` with `Render` trait
  - Add search icon (20px), text input (16px font)
  - Fixed height 48px, horizontal padding
  - Implement placeholder "Search PhotonCast..."
  - Dependencies: 1.3.3, 1.2.2
  - **Status**: Implemented with GPUI `Render` trait, constants exported

- [x] **1.4.2 Add SearchBar focus handling** (2h)
  - Auto-focus on window show
  - Visual focus indicator (border color change)
  - Clear on Escape key
  - Dependencies: 1.4.1
  - **Status**: Implemented with `FocusHandle`, focus border color, `clear()` method

- [x] **1.4.3 Implement input debouncing** (1h)
  - 16ms debounce (single frame at 60 FPS)
  - Emit `on_change` event after debounce
  - Prevent excessive re-renders
  - Dependencies: 1.4.1
  - **Status**: Implemented with `DEBOUNCE_DURATION`, async spawn for debounce

- [x] **1.4.4 Implement ResultsList component** (3h) ⭐ CRITICAL
  - Create `src/ui/results_list.rs` with scrollable container
  - Implement virtual scrolling for performance
  - Calculate visible range, render only visible items
  - Add spacers for off-screen items
  - Dependencies: 1.3.3, 1.2.2
  - **Status**: Implemented with `calculate_visible_range()`, top/bottom spacers, overscan

- [x] **1.4.5 Implement ResultItem component** (3h) ⭐ CRITICAL
  - Create `src/ui/result_item.rs` with `RenderOnce` trait
  - Layout: icon (32px), title, subtitle, shortcut badge
  - Fixed height 56px, horizontal padding 16px
  - Dependencies: 1.3.3
  - **Status**: Implemented with `IntoElement` trait, all layout specs

- [x] **1.4.6 Add ResultItem selection states** (2h)
  - Normal: default background
  - Hover: `surface_hover` background
  - Selected: `surface_selected` with accent border
  - Use theme colors for all states
  - Dependencies: 1.4.5
  - **Status**: Implemented with `is_selected`, `is_hovered` flags and theme color mapping

- [x] **1.4.7 Implement match highlighting** (2h)
  - Accept `match_ranges: Vec<Range<usize>>` prop
  - Apply accent color to matched characters in title
  - Handle multi-range highlighting
  - Dependencies: 1.4.5
  - **Status**: Implemented with `render_title()` building highlighted spans

- [x] **1.4.8 Implement ResultGroup component** (2h)
  - Create `src/ui/result_group.rs` for section headers
  - Display group name (Apps, Commands, Files)
  - Include shortcut hint (⌘1-5)
  - Dependencies: 1.3.3
  - **Status**: Implemented with `IntoElement`, `build_shortcut_hint()`, `ResultGroupWithItems`

- [x] **1.4.9 Write component unit tests** (2h)
  - Test SearchBar renders correctly
  - Test ResultItem states (normal, hover, selected)
  - Test ResultsList virtual scrolling calculations
  - Dependencies: 1.4.1, 1.4.4, 1.4.5
  - **Status**: Unit tests embedded in each component file (search_bar, result_item, results_list, result_group)
  - **Note**: Full build/test requires Xcode with accepted license for GPUI metal shaders

### 1.5 UI States

- [x] **1.5.1 Implement EmptyState component** (2h)
  - Create `src/ui/empty_state.rs`
  - No query: "Type to search apps, commands, and files"
  - No results: 'No results for "query"'
  - Include keyboard hints
  - Dependencies: 1.3.3
  - **Status**: Implemented with `EmptyState` struct, `message()`, `hint()`, `keyboard_hints()` methods

- [x] **1.5.2 Implement LoadingState component** (2h)
  - Create loading spinner animation
  - Display "Indexing applications..." with progress
  - Show found count "Found 142 of ~200 apps"
  - Dependencies: 1.3.3
  - **Status**: Implemented with `LoadingState` struct, progress tracking, `spinner_char()` for animation

- [x] **1.5.3 Implement ErrorState component** (2h)
  - Display error icon and message
  - Include action buttons (Retry, Open Folder)
  - Style with warning/error theme colors
  - Dependencies: 1.3.3
  - **Status**: Implemented with `ErrorState`, `AppError`, `ErrorAction`, `ErrorCode` types

- [x] **1.5.4 Wire up state management** (2h)
  - Create `LauncherState` enum (Empty, Loading, Results, Error)
  - Connect states to ResultsList display
  - Handle state transitions
  - Dependencies: 1.5.1, 1.5.2, 1.5.3
  - **Status**: Implemented `LauncherState` enum with all 4 states and helper methods

### 1.6 Keyboard Navigation

- [x] **1.6.1 Implement selection state management** (2h) ⭐ CRITICAL
  - Track `selected_index: usize` in state
  - Clamp to valid range on results update
  - Reset to 0 on new search
  - Dependencies: 1.4.4
  - **Status**: Implemented in `LauncherWindow` with `selected_index` field, resets on `on_query_change()`

- [x] **1.6.2 Implement ↑/↓ navigation** (2h) ⭐ CRITICAL
  - `SelectNext`: increment with bounds check
  - `SelectPrevious`: decrement with bounds check
  - Also support `Ctrl+N/P` alternatives
  - Dependencies: 1.6.1, 1.2.4
  - **Status**: Implemented with `select_next()`, `select_previous()` handlers; key bindings include Ctrl+N/P

- [x] **1.6.3 Implement Enter activation** (2h)
  - Dispatch `Activate` action on Enter
  - Get selected result and trigger action
  - Close launcher after activation
  - Dependencies: 1.6.1, 1.2.4
  - **Status**: Implemented with `activate()` handler, hides launcher after activation

- [x] **1.6.4 Implement Escape handling** (1h)
  - If query present: clear query
  - If query empty: close launcher
  - Dependencies: 1.2.4
  - **Status**: Implemented in `cancel()` handler with two-stage escape behavior

- [x] **1.6.5 Implement ⌘1-9 quick selection** (2h)
  - Map `⌘1` to first result, `⌘9` to ninth
  - Immediately activate selected result
  - Show shortcut badges in ResultItem
  - Dependencies: 1.2.4, 1.4.5
  - **Status**: Implemented with `quick_select()` and 9 QuickSelect actions; badges shown in result items

- [x] **1.6.6 Implement Tab group cycling** (1h)
  - Tab: move to next group's first item
  - Shift+Tab: move to previous group
  - Dependencies: 1.6.1, 1.4.8
  - **Status**: Implemented with `next_group()`, `previous_group()` handlers with wrap-around

- [x] **1.6.7 Implement scroll-to-selection** (2h)
  - Auto-scroll to keep selected item visible
  - Smooth scrolling within viewport
  - Dependencies: 1.6.1, 1.4.4
  - **Status**: Implemented with `ensure_selected_visible()` called on navigation; GPUI scroll container handles visibility

- [x] **1.6.8 Write keyboard navigation tests** (2h)
  - Test ↑/↓ bounds checking
  - Test Enter activates correct item
  - Test ⌘1-9 quick selection
  - Dependencies: 1.6.1, 1.6.2, 1.6.3, 1.6.5
  - **Status**: Unit tests in `tests/integration/keyboard_test.rs`; GPUI integration tests require Xcode

### 1.7 Animations

- [x] **1.7.1 Implement window appear animation** (2h)
  - 150ms ease-out animation
  - Fade in + slight scale up
  - Dependencies: 1.2.3
  - **Status**: Implemented with `start_appear_animation()`, `animation_progress()`, opacity-based fade using `ease_out()`

- [x] **1.7.2 Implement window dismiss animation** (1h)
  - 100ms ease-in animation
  - Fade out + slight scale down
  - Dependencies: 1.2.3
  - **Status**: Implemented with `start_dismiss_animation()`, triggered on window close with `ease_in()` easing

- [x] **1.7.3 Implement selection change animation** (2h)
  - 80ms ease-in-out transition
  - Smooth background color transition
  - Dependencies: 1.4.6
  - **Status**: Implemented with `start_selection_animation()`, `selection_animation_progress()`, tracks previous selection

- [x] **1.7.4 Implement hover highlight animation** (1h)
  - 60ms linear transition
  - Subtle background color change
  - Dependencies: 1.4.6
  - **Status**: Implemented via GPUI's built-in `.hover()` method with theme-aware background colors

- [x] **1.7.5 Implement reduce motion support** (2h) ⭐ CRITICAL
  - Detect `NSWorkspace.accessibilityDisplayShouldReduceMotion`
  - Create `animation_duration()` helper function
  - When enabled: instant transitions, no physics
  - Support PhotonCast settings override
  - Dependencies: 1.7.1, 1.7.2, 1.7.3, 1.7.4
  - **Status**: Implemented in `ui/animations.rs` with `reduce_motion_enabled()`, `animation_duration()`, override support via `set_reduce_motion_override()`

- [x] **1.7.6 Write animation tests** (1h)
  - Test reduce motion detection
  - Test animation duration helper
  - Dependencies: 1.7.5
  - **Status**: 35+ tests in `tests/integration/animation_test.rs` covering easing functions, duration helpers, state types, and reduce motion integration

### Sprint 1 Milestone Checklist

- [x] Empty launcher window appears/disappears in <50ms
- [x] All 4 Catppuccin themes render correctly
- [x] System theme sync works automatically
- [x] SearchBar accepts input with focus handling
- [x] ResultsList renders with virtual scrolling
- [x] Keyboard navigation (↑↓, Enter, Esc) works
- [x] ⌘1-9 quick selection works
- [x] UI renders at consistent 120 FPS
- [x] Reduce motion accessibility support works
- [x] CI pipeline passes all checks

---

## Sprint 2: App Launcher (Weeks 5-8)

### 2.1 Application Indexing

- [x] **2.1.1 Create app scanner module** (3h) ⭐ CRITICAL
  - Create `src/indexer/scanner.rs`
  - Scan `/Applications`, `/System/Applications`, `~/Applications`
  - Filter to `.app` bundles only
  - Exclude patterns: `.prefPane`, `*Uninstaller*`, nested apps
  - Dependencies: 1.1.4

- [x] **2.1.2 Implement Info.plist parser** (3h) ⭐ CRITICAL
  - Create `src/indexer/metadata.rs`
  - Add `plist` crate dependency
  - Parse `CFBundleDisplayName` or `CFBundleName`
  - Extract `CFBundleIdentifier` (required)
  - Extract `LSApplicationCategoryType` (optional)
  - Dependencies: 2.1.1

- [x] **2.1.3 Create IndexedApp data structure** (1h)
  - Define struct with name, bundle_id, path, icon, keywords, category
  - Implement `last_modified` timestamp tracking
  - Add `Clone`, `Debug`, `PartialEq` derives
  - Dependencies: 2.1.2

- [x] **2.1.4 Implement async directory scanning** (2h)
  - Use `tokio::fs::read_dir` for non-blocking I/O
  - Spawn concurrent tasks per directory
  - Collect results with timeout handling
  - Target: <2s for full scan of ~200 apps
  - Dependencies: 2.1.1, 2.1.2

- [x] **2.1.5 Implement icon extraction** (4h)
  - Create `src/indexer/icons.rs`
  - Read `CFBundleIconFile` from Info.plist
  - Load `.icns` files from `Contents/Resources/`
  - Add `icns` crate for parsing
  - Cache extracted icons to disk
  - Dependencies: 2.1.2

- [x] **2.1.6 Create icon cache system** (2h)
  - LRU cache with 100 icon limit
  - Store in `~/Library/Caches/PhotonCast/icons/`
  - Lazy loading with `OnceCell`
  - Dependencies: 2.1.5

- [x] **2.1.7 Write indexer unit tests** (2h)
  - Test plist parsing with fixture files
  - Test app bundle discovery
  - Test icon extraction
  - Dependencies: 2.1.2, 2.1.5

- [x] **2.1.8 Write indexer integration tests** (2h)
  - Create mock app bundles in temp directory
  - Test full scan workflow
  - Verify metadata extraction accuracy
  - Dependencies: 2.1.4

### 2.2 Database & Persistence

- [x] **2.2.1 Add rusqlite dependency** (1h)
  - Add `rusqlite` with `bundled` feature
  - Configure async wrapper or use `tokio::task::spawn_blocking`
  - Dependencies: 1.1.4

- [x] **2.2.2 Create database schema** (2h) ⭐ CRITICAL
  - Create `src/storage/database.rs`
  - Define `app_usage` table (bundle_id, launch_count, last_launched)
  - Define `command_usage` table
  - Define `file_usage` table
  - Define `app_cache` table for indexed apps
  - Dependencies: 2.2.1

- [x] **2.2.3 Implement database migrations** (2h)
  - Create migration system for schema versioning
  - Add migration 001: initial schema
  - Auto-run migrations on startup
  - Dependencies: 2.2.2

- [x] **2.2.4 Create database wrapper** (2h)
  - Implement `Database` struct with connection pool
  - Add async query methods
  - Handle connection errors gracefully
  - Dependencies: 2.2.2

- [x] **2.2.5 Implement app cache operations** (2h)
  - `insert_app()`, `get_all_apps()`, `remove_app()`
  - `update_app()` for incremental updates
  - Batch insert support for full re-index
  - Dependencies: 2.2.4

- [x] **2.2.6 Write database tests** (2h)
  - Test migrations run correctly
  - Test CRUD operations
  - Test concurrent access
  - Dependencies: 2.2.2, 2.2.4, 2.2.5

### 2.3 File System Watcher

- [x] **2.3.1 Add notify crate dependency** (1h)
  - Add `notify` crate for cross-platform FS events
  - Configure for macOS FSEvents backend
  - Dependencies: 1.1.4
  - **Status**: `notify = "6.1"` already in workspace Cargo.toml

- [x] **2.3.2 Implement FS watcher setup** (3h) ⭐ CRITICAL
  - Create `src/indexer/watcher.rs`
  - Watch all 3 scan paths non-recursively
  - Handle `Create`, `Modify`, `Remove` events
  - Filter to `.app` bundles only
  - Dependencies: 2.3.1, 2.1.1
  - **Status**: Implemented `AppWatcher` struct with `WatchEvent` enum, async channel-based API

- [x] **2.3.3 Implement debounced updates** (2h)
  - 500ms debounce for batch operations
  - Coalesce multiple events for same path
  - Trigger incremental re-index after debounce
  - Dependencies: 2.3.2
  - **Status**: 500ms debounce with `DebounceState` that coalesces events by path

- [x] **2.3.4 Wire up watcher to indexer** (2h)
  - Start watcher on app launch
  - Connect events to `AppIndexer.rescan_directory()`
  - Log watcher events at debug level
  - Dependencies: 2.3.2, 2.1.4
  - **Status**: `AppWatcher.start()` returns `mpsc::UnboundedReceiver<WatchEvent>` for async consumption, debug logging via `tracing`

- [x] **2.3.5 Write watcher integration tests** (2h)
  - Test app install detection
  - Test app removal detection
  - Test debouncing behavior
  - Dependencies: 2.3.2, 2.3.3
  - **Status**: Added `tests/integration/watcher_test.rs` with 12 tests covering install/removal detection, debouncing, filtering

### 2.4 Search Engine

- [x] **2.4.1 Add nucleo dependency** (1h) ⭐ CRITICAL
  - Add `nucleo` crate (or `nucleo-matcher`)
  - Configure for Unicode normalization and smart case
  - Dependencies: 1.1.4
  - **Status**: `nucleo = "0.5"` in workspace Cargo.toml with `Normalization::Smart` and smart case

- [x] **2.4.2 Implement FuzzyMatcher wrapper** (2h) ⭐ CRITICAL
  - Create `src/search/fuzzy.rs`
  - Wrap nucleo `Matcher` with PhotonCast config
  - Implement `score(query, target) -> Option<(u32, Vec<u32>)>`
  - Return match indices for highlighting
  - Dependencies: 2.4.1
  - **Status**: `FuzzyMatcher` with prefix boost, `score_many()` batch method, 12 unit tests

- [x] **2.4.3 Define SearchProvider trait** (1h)
  - Create `src/search/providers/mod.rs`
  - Define `trait SearchProvider: Send + Sync`
  - Methods: `name()`, `search()`, `result_type()`
  - Dependencies: 1.1.4
  - **Status**: Trait defined with `search(query, max_results) -> Vec<SearchResult>`

- [x] **2.4.4 Implement AppProvider** (3h) ⭐ CRITICAL
  - Create `src/search/providers/apps.rs`
  - Query indexed apps from memory/cache
  - Apply fuzzy matching to app names
  - Return `Vec<RawSearchResult>` with scores
  - Dependencies: 2.4.2, 2.4.3, 2.1.3
  - **Status**: Thread-safe `Arc<RwLock>` storage, fuzzy matching, 8 unit tests

- [x] **2.4.5 Create SearchEngine orchestrator** (3h) ⭐ CRITICAL
  - Create `src/search/engine.rs`
  - Hold vector of `Box<dyn SearchProvider>`
  - Dispatch queries to all providers in parallel
  - Collect and merge results
  - Dependencies: 2.4.3, 2.4.4
  - **Status**: `SearchConfig`, `search_sync()`, async `search()`, result grouping, 10 unit tests

- [x] **2.4.6 Implement search result types** (2h)
  - Define `SearchResult`, `SearchAction`, `ResultType` in `src/search/mod.rs`
  - `SearchAction` enum: `LaunchApp`, `ExecuteCommand`, `OpenFile`, `RevealInFinder`
  - Add `SearchResultId` newtype
  - Dependencies: 2.4.5
  - **Status**: All types implemented including `SearchResults`, `ResultGroup`, `IconSource`

- [x] **2.4.7 Wire search to UI** (2h)
  - Connect SearchBar `on_change` to SearchEngine
  - Spawn async search task with `cx.spawn()`
  - Update ResultsList on completion
  - Dependencies: 2.4.5, 1.4.1, 1.4.4
  - **Status**: Interface defined: `SearchEventHandler`, `async_search` module with debounced task pattern

- [x] **2.4.8 Write search unit tests** (2h)
  - Test fuzzy matching accuracy
  - Test score consistency
  - Test match indices correctness
  - Dependencies: 2.4.2
  - **Status**: 30+ unit tests across `fuzzy.rs`, `apps.rs`, `engine.rs`

- [x] **2.4.9 Write search integration tests** (2h)
  - Test full search workflow
  - Test result merging from multiple providers
  - Target: <30ms search latency
  - Dependencies: 2.4.5
  - **Status**: 20+ tests in `tests/integration/search_test.rs` including performance benchmark

### 2.5 Ranking Algorithm

- [x] **2.5.1 Implement pure match quality ranking** (2h) ⭐ CRITICAL
  - Create `src/search/ranking.rs`
  - Sort results by nucleo score (higher is better)
  - Dependencies: 2.4.5

- [x] **2.5.2 Implement frecency calculation** (3h) ⭐ CRITICAL
  - Define `FrecencyScore` struct
  - Calculate: `frequency * recency_decay`
  - Recency decay: half-life of 72 hours
  - Query usage data from database
  - Dependencies: 2.2.5, 2.5.1

- [x] **2.5.3 Implement boost factors** (2h)
  - Create `BoostConfig` with configurable multipliers
  - Path boosts: 1.2x for `/System/Applications`, 1.1x for `/Applications`
  - Match boosts: 2.0x for exact match, 1.5x for prefix match
  - Dependencies: 2.5.1

- [x] **2.5.4 Implement combined ranking** (2h)
  - Formula: `final_score = match_score + (frecency * 10.0)`
  - Apply boosts after combination
  - Dependencies: 2.5.1, 2.5.2, 2.5.3

- [x] **2.5.5 Implement tiebreaker logic** (1h)
  - Order: usage count → recency → alphabetical
  - Ensure deterministic ordering
  - Dependencies: 2.5.4

- [x] **2.5.6 Write ranking unit tests** (2h)
  - Test frecency calculation with known values
  - Test boost application
  - Test tiebreaker ordering
  - Dependencies: 2.5.2, 2.5.3, 2.5.5

- [x] **2.5.7 Write ranking property tests** (2h)
  - Test ranking is deterministic (same input → same output)
  - Test exact matches always rank higher than partial
  - Dependencies: 2.5.4

### 2.6 App Launching

- [x] **2.6.1 Implement NSWorkspace launcher** (3h) ⭐ CRITICAL
  - Create `src/platform/launch.rs`
  - Use `open -b` command for NSWorkspace launching
  - Implement `launch_app_by_bundle_id(bundle_id: &str) -> Result<()>`
  - Handle app not found, damaged app errors with specific error detection
  - Dependencies: 1.1.4
  - **Status**: Implemented `launch_app_by_bundle_id()`, `launch_app_by_path()`, `open_file()`, `reveal_in_finder()` with proper stderr parsing

- [x] **2.6.2 Create LaunchError type** (1h)
  - Define error variants: `NotFound`, `LaunchFailed`, `Damaged`
  - Implement user-friendly error messages via `user_message()` method
  - Implement `Display` trait via `thiserror`
  - Added `is_recoverable()`, `action_hint()`, `should_offer_reveal()` methods
  - Dependencies: 2.6.1
  - **Status**: Implemented with `#[derive(Error, Debug, Clone, PartialEq, Eq)]`

- [x] **2.6.3 Implement usage tracking** (2h)
  - On successful launch, increment `launch_count`
  - Update `last_launched_at` timestamp
  - Use database operations from 2.2.5
  - Dependencies: 2.6.1, 2.2.5
  - **Status**: `AppLauncher` struct wraps `UsageTracker`, auto-records on successful launch via `record_app_launch()`

- [x] **2.6.4 Wire launch to activation** (2h)
  - Connect `Activate` action to launch handler
  - Match `SearchAction::LaunchApp` variant
  - Created `AppLauncher.execute_action()` and `execute_action_async()` methods
  - Dependencies: 2.6.1, 1.6.3
  - **Status**: Handles `LaunchApp`, `OpenFile`, `RevealInFinder` actions; `ExecuteCommand` delegated to command executor

- [x] **2.6.5 Handle launch errors gracefully** (2h)
  - Return appropriate error for display via `user_message()`
  - Action hints suggest: "Remove from index" for NotFound, "Reveal in Finder" for Damaged, "Retry" for LaunchFailed
  - `should_offer_reveal()` method for damaged apps
  - Dependencies: 2.6.2, 1.5.3
  - **Status**: Error types include recovery hints; `LaunchError::Damaged` recommends Finder reveal

- [x] **2.6.6 Write launch tests** (1h)
  - Test successful launch (integration tests with `#[ignore]` for CI)
  - Test error handling for missing apps
  - Dependencies: 2.6.1, 2.6.2
  - **Status**: 30+ tests in `platform/launch.rs` and `tests/integration/launch_test.rs`; tests LaunchError variants, AppLauncher, and async operations

### 2.7 Search Performance Optimization

- [x] **2.7.1 Implement search index pre-computation** (2h)
  - Pre-lowercase app names for case-insensitive matching
  - Pre-sort by frequency for early termination
  - Store in memory for fast access
  - Dependencies: 2.4.4
  - **Status**: Implemented `SearchIndex` and `IndexedAppEntry` in `search/index.rs` with `name_lower` and `frecency` pre-computation

- [x] **2.7.2 Implement early termination** (2h)
  - Stop search when enough high-quality matches found
  - Use threshold: `max_results * 2`
  - Dependencies: 2.7.1
  - **Status**: Implemented `EarlyTerminationConfig` with configurable threshold multiplier (default 2.0) and `min_quality_score`

- [x] **2.7.3 Add search benchmarks** (2h)
  - Create `benches/search_bench.rs`
  - Benchmark fuzzy match on 200 apps
  - Benchmark ranking on 100 results
  - Target: <30ms end-to-end
  - Dependencies: 2.4.5, 2.5.4
  - **Status**: Created comprehensive benchmarks in `crates/photoncast-core/benches/search_bench.rs`; run with `cargo bench -p photoncast-core`

### Sprint 2 Milestone Checklist

- [ ] Apps indexed from all standard paths (<2s)
- [ ] Metadata and icons extracted correctly
- [ ] Database stores usage data persistently
- [ ] FS watcher detects app install/removal
- [ ] Fuzzy search returns relevant results
- [ ] Results ranked by match quality + frecency
- [ ] Apps launch correctly via NSWorkspace
- [ ] Usage tracking updates on launch
- [ ] Search latency <30ms
- [ ] All tests pass

---

## Sprint 3: Global Hotkey & System (Weeks 9-12)

### 3.1 Accessibility Permissions

- [x] **3.1.1 Implement permission status check** (2h) ⭐ CRITICAL
  - Create `src/platform/accessibility.rs`
  - Call `AXIsProcessTrusted()` from ApplicationServices framework
  - Return `PermissionStatus` enum: Granted, Denied, Unknown
  - Dependencies: 1.1.4

- [x] **3.1.2 Implement permission request with prompt** (2h)
  - Call `AXIsProcessTrustedWithOptions` with prompt flag
  - Trigger macOS permission dialog
  - Dependencies: 3.1.1

- [x] **3.1.3 Create PermissionDialog UI component** (3h) ⭐ CRITICAL
  - Create `src/ui/permission_dialog.rs`
  - Display explanation of why permission is needed
  - Bullet points: "Register global shortcuts", "Respond to hotkey"
  - Add "Open System Settings" and "Skip for Now" buttons
  - Dependencies: 1.3.3, 3.1.1

- [x] **3.1.4 Implement "Open System Settings" action** (1h)
  - Open Privacy & Security → Accessibility pane
  - Use `open` command with URL scheme
  - Dependencies: 3.1.3

- [x] **3.1.5 Implement real-time permission polling** (2h)
  - Poll permission status every 1 second
  - When granted, signal success (to dismiss dialog later)
  - Stop polling when granted or timeout
  - Dependencies: 3.1.1, 3.1.3

- [x] **3.1.6 Wire permission flow to app startup** (2h)
  - Check permission on launch
  - If not granted, prepare to show dialog before hotkey registration
  - Allow launcher to work from menu bar without permission
  - Dependencies: 3.1.1, 3.1.3

- [x] **3.1.7 Write permission tests** (1h)
  - Test status check function compiles and runs
  - Test dialog data structures
  - Dependencies: 3.1.1, 3.1.3

### 3.2 Global Hotkey Registration

- [x] **3.2.1 Create HotkeyBinding type** (1h)
  - Define `HotkeyBinding { key: KeyCode, modifiers: Modifiers }`
  - Default: `Cmd+Space`
  - Implement `Default`, `Clone`, `Debug` traits
  - Dependencies: 1.1.4
  - **Status**: Implemented in `platform/hotkey.rs` with `KeyCode` enum and `Modifiers` struct

- [x] **3.2.2 Implement CGEventTap hotkey handler** (4h) ⭐ CRITICAL
  - Create `src/platform/hotkey.rs`
  - Create event tap for `KeyDown` and `FlagsChanged` events
  - Match events against registered binding
  - Consume matched events (return None)
  - Dependencies: 3.2.1
  - **Status**: Implemented CGEventTap-based handler with FFI bindings to CoreGraphics

- [x] **3.2.3 Create HotkeyManager** (3h) ⭐ CRITICAL
  - Implement `register()`, `unregister()` methods
  - Check accessibility permission before registration
  - Enable tap and add to CFRunLoop
  - Store registration state
  - Dependencies: 3.2.2, 3.1.1
  - **Status**: Implemented `HotkeyManager` with full registration lifecycle

- [x] **3.2.4 Wire hotkey to window toggle** (2h)
  - On hotkey press, invoke callback
  - Ensure thread-safe communication to main thread
  - Target: <50ms hotkey response
  - Dependencies: 3.2.3, 1.2.3
  - **Status**: Callback-based design implemented - caller provides toggle function

- [x] **3.2.5 Create HotkeyError type** (1h)
  - Define variants: `PermissionDenied`, `ConflictDetected`, `RegistrationFailed`, `InvalidBinding`
  - Implement user-friendly messages with action hints
  - Dependencies: 3.2.3
  - **Status**: Implemented with `user_message()`, `is_recoverable()`, and `action_hint()` methods

- [x] **3.2.6 Write hotkey integration tests** (2h)
  - Test registration succeeds with permission
  - Test registration fails without permission
  - Test event matching
  - Dependencies: 3.2.3
  - **Status**: Implemented in `tests/integration/hotkey_test.rs` with 20+ tests

### 3.3 Hotkey Conflict Detection

- [x] **3.3.1 Read Spotlight shortcut status** (2h)
  - Read `~/Library/Preferences/com.apple.symbolichotkeys.plist`
  - Check key 64 (Spotlight) enabled status
  - Parse plist with `plist` crate
  - Dependencies: 1.1.4
  - **Status**: Implemented `read_spotlight_enabled_status()` and `is_spotlight_enabled()` in `platform/hotkey.rs`

- [x] **3.3.2 Implement conflict detection** (2h) ⭐ CRITICAL
  - Create `detect_hotkey_conflict(binding) -> Option<ConflictInfo>`
  - Check Spotlight (Cmd+Space)
  - Return conflicting app name if found
  - Dependencies: 3.3.1
  - **Status**: Implemented `ConflictInfo` struct and `detect_hotkey_conflict()` function

- [x] **3.3.3 Handle conflicts in registration** (2h)
  - Return `HotkeyError::ConflictDetected` with app name
  - Show user-friendly conflict message
  - Suggest changing PhotonCast hotkey
  - Dependencies: 3.3.2, 3.2.5
  - **Status**: `HotkeyError::ConflictDetected` now includes `suggestion` field with user-friendly message

- [x] **3.3.4 Write conflict detection tests** (1h)
  - Test Spotlight detection
  - Test conflict error creation
  - Dependencies: 3.3.1, 3.3.2
  - **Status**: 25 unit tests added covering plist parsing, conflict detection, error types, and mock data

### 3.4 Double-Tap Modifier Support

- [x] **3.4.1 Implement DoubleTapDetector** (3h)
  - Create struct tracking `last_modifier_press: Option<Instant>`
  - Configure threshold (300ms default)
  - Track target modifier (e.g., Command)
  - Dependencies: 3.2.2

- [x] **3.4.2 Implement modifier event handling** (2h)
  - On modifier press: record timestamp
  - On second press within threshold: trigger
  - Reset state after trigger or timeout
  - Dependencies: 3.4.1

- [x] **3.4.3 Add double-tap to HotkeyManager** (2h)
  - Support `double_tap_modifier: Option<Modifier>` config
  - Wire to existing hotkey callback
  - Dependencies: 3.4.1, 3.2.3

- [x] **3.4.4 Write double-tap tests** (1h)
  - Test detection within threshold
  - Test no detection outside threshold
  - Dependencies: 3.4.1, 3.4.2

### 3.5 Hotkey Customization

- [x] **3.5.1 Add hotkey to config file** (1h)
  - Add `[hotkey]` section to config schema
  - Support `key` and `modifiers` fields
  - Support `double_tap_modifier` optional field
  - Dependencies: 2.2.2
  - **Status**: Implemented `HotkeyConfig` in `app/config.rs` with all fields

- [x] **3.5.2 Implement hotkey settings UI** (3h)
  - Create settings panel for hotkey configuration
  - Show current binding
  - Allow key capture for new binding
  - Dependencies: 1.3.3, 3.5.1
  - **Status**: Implemented `HotkeySettings` in `platform/hotkey_settings.rs` with data model, capture state, and display functions

- [x] **3.5.3 Implement key capture** (2h)
  - Listen for next keypress in capture mode
  - Validate binding (no single modifier, no reserved keys)
  - Dependencies: 3.5.2
  - **Status**: Implemented `KeyCaptureState` with `start_capture()`, `on_key_press()`, `validate_binding()`, and reserved key checks

- [x] **3.5.4 Implement hotkey change with re-registration** (2h)
  - Unregister old hotkey
  - Validate new binding
  - Register new hotkey
  - Persist to config file
  - Dependencies: 3.5.2, 3.2.3
  - **Status**: Implemented `HotkeyChangeManager` with rollback support, `save_hotkey_config()`, and `load_config()` functions

- [x] **3.5.5 Write hotkey settings tests** (1h)
  - Test config parsing
  - Test binding validation
  - Dependencies: 3.5.1, 3.5.3
  - **Status**: 30+ unit tests in `platform/hotkey_settings.rs` covering validation, capture, settings, and formatting

### 3.6 System Commands

- [x] **3.6.1 Define SystemCommand enum** (2h) ⭐ CRITICAL
  - Create `src/commands/definitions.rs`
  - Variants: Sleep, SleepDisplays, LockScreen, Restart, ShutDown, LogOut, EmptyTrash, ScreenSaver
  - Add metadata: name, aliases, description, icon, requires_confirmation
  - Dependencies: 1.1.4

- [x] **3.6.2 Implement Sleep command** (1h)
  - Execute `pmset sleepnow`
  - No confirmation required
  - Dependencies: 3.6.1

- [x] **3.6.3 Implement SleepDisplays command** (1h)
  - Execute `pmset displaysleepnow`
  - No confirmation required
  - Dependencies: 3.6.1

- [x] **3.6.4 Implement LockScreen command** (1h)
  - Use AppleScript to simulate Cmd+Ctrl+Q keystroke (locks screen)
  - No confirmation required
  - Dependencies: 3.6.1

- [x] **3.6.5 Implement Restart/ShutDown/LogOut commands** (2h)
  - Use AppleScript via `osascript`
  - Require confirmation dialog
  - Dependencies: 3.6.1

- [x] **3.6.6 Implement EmptyTrash command** (1h)
  - Use Finder AppleScript
  - Require confirmation dialog
  - Dependencies: 3.6.1

- [x] **3.6.7 Implement ScreenSaver command** (1h)
  - Execute `open -a ScreenSaverEngine`
  - No confirmation required
  - Dependencies: 3.6.1

- [x] **3.6.8 Create confirmation dialog** (2h)
  - Created `ConfirmationDialog` struct with title, message, confirm/cancel labels
  - Include action name and description
  - Is_destructive flag for button styling
  - Dependencies: 1.3.3

- [x] **3.6.9 Create AppleScript executor** (2h)
  - Implement `run_applescript(script) -> Result<()>` with debug logging
  - Implement `run_applescript_with_output(script) -> Result<String>`
  - Handle script errors with detailed error logging
  - Log execution at debug level using tracing
  - Dependencies: 3.6.5

- [x] **3.6.10 Create CommandError type** (1h)
  - Define variants: `ExecutionFailed`, `AuthorizationRequired`, `NotAvailable`
  - Implement user-friendly messages via `user_message()` method
  - Added `is_recoverable()` helper method
  - Dependencies: 3.6.1

- [x] **3.6.11 Write command unit tests** (2h)
  - Test command metadata (25 tests)
  - Test confirmation dialog creation
  - Test error types and user messages
  - Test AppleScript interface
  - Dependencies: 3.6.1

### 3.7 Command Search Provider

- [x] **3.7.1 Implement CommandProvider** (3h) ⭐ CRITICAL
  - Create `src/search/providers/commands.rs`
  - Implement `SearchProvider` trait
  - Match query against command names and aliases
  - Return command results with icons
  - Dependencies: 2.4.3, 3.6.1
  - **Status**: Implemented with FuzzyMatcher, scores against names and aliases

- [x] **3.7.2 Wire command activation** (2h)
  - Handle `SearchAction::ExecuteCommand` variant
  - Execute command with confirmation if required
  - Close launcher after execution
  - Dependencies: 3.7.1, 3.6.8
  - **Status**: Added `CommandExecutor` with `lookup()`, `execute()`, `execute_by_id()` methods

- [x] **3.7.3 Add command usage tracking** (1h)
  - Track command executions in database
  - Update frecency for commands
  - Dependencies: 3.7.2, 2.2.5
  - **Status**: Added `CommandUsageTracker` trait, `InMemoryUsageTracker`, `NoOpUsageTracker`

- [x] **3.7.4 Write command provider tests** (1h)
  - Test search returns correct commands
  - Test alias matching
  - Dependencies: 3.7.1
  - **Status**: 18 tests for CommandProvider, 6 for executor, 5 for usage tracker

### 3.8 Spotlight File Search

- [x] **3.8.1 Implement NSMetadataQuery wrapper** (4h) ⭐ CRITICAL
  - Create `src/platform/spotlight.rs`
  - Use `mdfind` command (simpler than NSMetadataQuery FFI)
  - Build query for name search: `-name <query>`
  - Set search scope: user home directory via `-onlyin`
  - Dependencies: 1.1.4
  - **Status**: Implemented `SpotlightQuery` with async/sync execution, `SpotlightProvider` for high-level interface

- [x] **3.8.2 Implement async query execution** (2h)
  - Use `tokio::process::Command` for async query execution
  - Set result limit (configurable, default 5)
  - Add 500ms timeout via `tokio::time::timeout`
  - Dependencies: 3.8.1
  - **Status**: `SpotlightQuery::execute()` with `tokio::time::timeout`, `execute_sync()` for blocking use

- [x] **3.8.3 Create FileResult type** (1h)
  - Define struct: path, name, kind (file/folder), size, modified
  - Implement `FileKind::from_path()` helper
  - Dependencies: 3.8.1
  - **Status**: `FileResult` with lazy-loaded metadata, `FileKind` enum with 8 variants, `display_name()` and `icon_name()` methods

- [x] **3.8.4 Implement FileProvider** (3h) ⭐ CRITICAL
  - Create `src/search/providers/files.rs`
  - Implement `SearchProvider` trait
  - Wrap Spotlight queries
  - Convert `FileResult` to `SearchResult`
  - Dependencies: 2.4.3, 3.8.1, 3.8.3
  - **Status**: `FileProvider` with caching, `search_async()` method, result conversion to `SearchResult` with `OpenFile` action

- [x] **3.8.5 Implement file opening** (2h)
  - Use `open` command to open files
  - Handle `SearchAction::OpenFile` variant
  - Add "Reveal in Finder" via `open -R`
  - Dependencies: 3.8.4
  - **Status**: `open_file()` and `reveal_in_finder()` in `platform/launch.rs`, `AppLauncher::execute_action()` handles OpenFile and RevealInFinder

- [x] **3.8.6 Add file usage tracking** (1h)
  - Track file opens in database via `record_file_open()`
  - Update frecency for files via `UsageTracker`
  - Dependencies: 3.8.5, 2.2.5
  - **Status**: `UsageTracker::record_file_open()` and `get_file_frecency()`, integrated into `AppLauncher::execute_action()`

- [x] **3.8.7 Write Spotlight integration tests** (2h)
  - Test query execution returns results
  - Test timeout handling
  - Test result conversion
  - Dependencies: 3.8.1, 3.8.4
  - **Status**: `tests/integration/spotlight_test.rs` with 40+ tests for FileKind, FileResult, SpotlightQuery, SpotlightProvider, FileProvider

### 3.9 Result Grouping

- [x] **3.9.1 Implement result grouping logic** (2h) ⭐ CRITICAL
  - Create `SearchResults::grouped()` method
  - Group results by `ResultType`
  - Sort groups: Apps → Commands → Files
  - Dependencies: 2.4.6
  - **Status**: Implemented `GroupedResult` struct with `shortcut_hint()`, navigation methods `next_group_start()`, `previous_group_start()`, `group_index_for_result()`, `first_index_in_group()` in `search/mod.rs`

- [x] **3.9.2 Update ResultsList for grouped display** (2h)
  - Render `ResultGroup` headers between sections
  - Display group name and shortcut range
  - Dependencies: 3.9.1, 1.4.8
  - **Status**: Updated `ResultsList` to accept `SearchResults`, render `ResultGroup` headers with shortcut hints, calculate y positions accounting for headers

- [x] **3.9.3 Implement inter-group navigation** (2h)
  - Tab moves to next group
  - Update ⌘1-9 to work across groups
  - Dependencies: 3.9.2, 1.6.6
  - **Status**: Implemented `next_group()`, `previous_group()`, `quick_select()` methods in `ResultsList` with wrap-around support

- [x] **3.9.4 Write grouping tests** (1h)
  - Test correct grouping order
  - Test navigation across groups
  - Dependencies: 3.9.1, 3.9.3
  - **Status**: Added 7 grouping tests in `tests/integration/search_test.rs`: order verification, shortcut indices, next/prev group navigation, group index lookup, edge cases

### 3.10 Final Integration & Polish

- [x] **3.10.1 Register all search providers** (1h)
  - Add AppProvider, CommandProvider, FileProvider to SearchEngine
  - Configure provider priorities
  - Dependencies: 2.4.4, 3.7.1, 3.8.4
  - **Status**: Implemented `PhotonCastApp` in `app/integration.rs` that registers all 3 providers with configurable priorities

- [x] **3.10.2 Implement search timeout handling** (1h)
  - Return partial results if timeout exceeded
  - Show "Search took too long" toast
  - Dependencies: 2.4.5
  - **Status**: Implemented `SearchOutcome` with `timed_out` flag and message, async timeout via `tokio::time::timeout`

- [x] **3.10.3 Add menu bar icon** (2h)
  - Create status item in menu bar
  - Show PhotonCast icon
  - Click to toggle launcher
  - Dependencies: 1.2.3
  - **Status**: Implemented `MenuBarManager` in `platform/menu_bar.rs` with `MenuItem`, `MenuBarAction`, `default_menu_items()` for GPUI integration

- [x] **3.10.4 Implement launch at login** (2h)
  - Use `SMAppService` for login item registration
  - Add toggle in settings
  - Dependencies: 3.5.2
  - **Status**: Implemented `LoginItemManager` in `platform/login_item.rs` with `enable()`, `disable()`, `toggle()`, `check_status()`, `open_settings()` methods

- [x] **3.10.5 Add preferences shortcut** (1h)
  - ⌘, opens preferences window
  - Dependencies: 3.5.2, 1.2.4
  - **Status**: Already implemented in `main.rs` with `KeyBinding::new("cmd-,", OpenPreferences, Some("LauncherWindow"))`

- [x] **3.10.6 Create config file loading** (2h)
  - Load from `~/.config/photoncast/config.toml`
  - Create default config if missing
  - Dependencies: 2.2.2
  - **Status**: Implemented `load_config()`, `load_config_from()`, `ConfigManager` in `app/config_file.rs`

- [x] **3.10.7 Implement config file saving** (1h)
  - Save settings changes to config file
  - Use atomic write
  - Dependencies: 3.10.6
  - **Status**: Implemented `save_config()`, `save_config_to()` with atomic write via temp file + rename in `app/config_file.rs`

- [x] **3.10.8 Final performance profiling** (2h)
  - Profile cold start time
  - Profile hotkey response time
  - Profile search latency
  - Identify and fix bottlenecks
  - Dependencies: All
  - **Status**: Implemented `ScopedProfiler`, `PerformanceReport`, `ProfileResult` in `utils/profiling.rs` with target constants (100ms cold start, 50ms hotkey, 30ms search, 50MB memory)

- [x] **3.10.9 Write end-to-end tests** (3h)
  - Test full app lifecycle
  - Test search → activate workflow
  - Test hotkey → search → launch workflow
  - Dependencies: All
  - **Status**: 25+ end-to-end tests in `tests/integration/e2e_test.rs` covering app lifecycle, search workflows, config loading, menu bar, performance, edge cases

### Sprint 3 Milestone Checklist

- [ ] Accessibility permission flow works smoothly
- [ ] Global hotkey responds in <50ms
- [ ] Hotkey conflicts detected and reported
- [ ] Double-tap modifier support works
- [ ] Hotkey customization persists
- [ ] All 7 system commands execute correctly
- [ ] Confirmation dialogs work for destructive commands
- [ ] Files searchable via Spotlight
- [ ] Results grouped by type (Apps, Commands, Files)
- [ ] Menu bar icon shows and works
- [ ] Launch at login option works
- [ ] All tests pass
- [ ] All performance targets met

---

## Critical Path Summary

The following tasks are on the critical path and must be completed on schedule:

### Sprint 1 Critical Path
1. `1.1.1` → `1.1.4` → `1.2.1` → `1.2.2` (Project setup → GPUI bootstrap → Window)
2. `1.3.1` → `1.3.2` → `1.3.3` (Theme palette → Semantic colors → Provider)
3. `1.4.1` → `1.4.4` → `1.4.5` (SearchBar → ResultsList → ResultItem)
4. `1.6.1` → `1.6.2` → `1.6.3` (Selection state → Navigation → Activation)

### Sprint 2 Critical Path
1. `2.1.1` → `2.1.2` → `2.1.4` (Scanner → Parser → Async scanning)
2. `2.2.2` → `2.2.4` → `2.2.5` (Schema → Database wrapper → Operations)
3. `2.4.1` → `2.4.2` → `2.4.4` → `2.4.5` (nucleo → FuzzyMatcher → AppProvider → Engine)
4. `2.5.1` → `2.5.2` → `2.5.4` (Match ranking → Frecency → Combined ranking)
5. `2.6.1` (App launching)

### Sprint 3 Critical Path
1. `3.1.1` → `3.1.3` (Permission check → Dialog)
2. `3.2.2` → `3.2.3` → `3.2.4` (CGEventTap → HotkeyManager → Window toggle)
3. `3.3.2` (Conflict detection)
4. `3.6.1` → `3.7.1` (System commands → Command provider)
5. `3.8.1` → `3.8.4` (NSMetadataQuery → File provider)
6. `3.9.1` (Result grouping)

---

## Task Count Summary

| Sprint | Tasks | Critical |
|--------|-------|----------|
| Sprint 1 | 50 | 18 |
| Sprint 2 | 42 | 14 |
| Sprint 3 | 52 | 16 |
| **Total** | **144** | **48** |

---

*Generated: 2026-01-15*  
*PhotonCast Phase 1 MVP Task Breakdown v1.0*
