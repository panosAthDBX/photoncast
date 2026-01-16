# PhotonCast Phase 1 MVP Specification

> Lightning-fast macOS launcher built in pure Rust

**Version:** 0.1.0-alpha  
**Timeline:** 12 weeks (3 sprints)  
**Document Date:** 2026-01-15

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [UI/UX Specification](#3-uiux-specification)
4. [Core Systems](#4-core-systems)
5. [Platform Integration](#5-platform-integration)
6. [Data Models](#6-data-models)
7. [Error Handling](#7-error-handling)
8. [Performance](#8-performance)
9. [Testing Strategy](#9-testing-strategy)
10. [Implementation Phases](#10-implementation-phases)
11. [Open Questions / Risks](#11-open-questions--risks)

---

## 1. Overview

### 1.1 Project Summary

PhotonCast is a native macOS application launcher built in Rust using GPUI for GPU-accelerated rendering at 120 FPS. It provides instant access to applications, files, and system commands without subscription fees, AI features, or privacy compromises.

### 1.2 Goals

| Goal | Description |
|------|-------------|
| **Performance** | Sub-50ms hotkey response, <30ms search latency |
| **Privacy** | 100% local, zero telemetry, no cloud sync |
| **Native** | Pure Rust with GPUI, no Electron bloat |
| **Simplicity** | Core launcher features without feature creep |
| **Accessibility** | VoiceOver support, keyboard-first design |

### 1.3 Non-Goals (Phase 1)

| Non-Goal | Rationale |
|----------|-----------|
| AI/LLM features | Focus on core utility, not AI gimmicks |
| Cloud sync | Privacy-first, local-only architecture |
| Extension system | Deferred to Phase 3 |
| Clipboard history | Deferred to Phase 2 |
| Window management | Deferred to Phase 2 |
| Calculator | Deferred to Phase 2 |

### 1.4 Target Users

- **macOS power users** who use their launcher 50+ times daily
- **Developers** needing quick access to projects and terminals
- **Privacy-conscious users** who avoid cloud-dependent tools
- **Keyboard enthusiasts** who minimize mouse usage

### 1.5 Tech Stack Summary

| Layer | Technology | Purpose |
|-------|------------|---------|
| Language | Rust 2021 (MSRV 1.75+) | Performance, safety |
| GUI | GPUI + gpui-component | GPU-accelerated 120 FPS |
| Async | Tokio | Non-blocking I/O |
| Search | nucleo | High-performance fuzzy matching |
| Storage | rusqlite | Local database |
| macOS | objc2, cocoa, core-foundation | Native integration |
| Theming | Catppuccin | 4 flavor color system |
| Errors | thiserror + anyhow | Idiomatic error handling |

---

## 2. Architecture

### 2.1 System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         PhotonCast Application                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        GPUI Layer                                │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐  │   │
│  │  │  Launcher   │  │   Search     │  │    Results List        │  │   │
│  │  │   Window    │  │    Bar       │  │    (virtualized)       │  │   │
│  │  └─────────────┘  └──────────────┘  └────────────────────────┘  │   │
│  │                                                                  │   │
│  │  ┌──────────────────────────────────────────────────────────┐   │   │
│  │  │                  Theme System (Catppuccin)                │   │   │
│  │  └──────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Core Services                               │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐  │   │
│  │  │   Search    │  │     App      │  │    System Commands     │  │   │
│  │  │   Engine    │  │   Indexer    │  │      Executor          │  │   │
│  │  └──────┬──────┘  └──────┬───────┘  └────────────────────────┘  │   │
│  │         │                │                                       │   │
│  │  ┌──────▼──────┐  ┌──────▼───────┐  ┌────────────────────────┐  │   │
│  │  │   nucleo    │  │   rusqlite   │  │   Spotlight Query      │  │   │
│  │  │   matcher   │  │   database   │  │   (NSMetadataQuery)    │  │   │
│  │  └─────────────┘  └──────────────┘  └────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    Platform Layer (macOS)                        │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐  │   │
│  │  │   Global    │  │ Accessibility│  │     NSWorkspace        │  │   │
│  │  │   Hotkey    │  │  Permissions │  │   (app launching)      │  │   │
│  │  └─────────────┘  └──────────────┘  └────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Module Structure

```
photoncast/
├── Cargo.toml
├── src/
│   ├── main.rs                     # Application entry, GPUI bootstrap
│   ├── lib.rs                      # Library root for testing
│   │
│   ├── app/                        # Application lifecycle
│   │   ├── mod.rs
│   │   ├── state.rs                # Global application state
│   │   ├── config.rs               # Configuration management
│   │   └── actions.rs              # GPUI action definitions
│   │
│   ├── ui/                         # GPUI views and components
│   │   ├── mod.rs
│   │   ├── launcher.rs             # Main launcher window
│   │   ├── search_bar.rs           # Search input component
│   │   ├── results_list.rs         # Results display
│   │   ├── result_item.rs          # Individual result row
│   │   ├── result_group.rs         # Section grouping component
│   │   ├── empty_state.rs          # No results / loading states
│   │   └── permission_dialog.rs    # Accessibility permission UI
│   │
│   ├── search/                     # Search engine
│   │   ├── mod.rs
│   │   ├── engine.rs               # Search orchestration
│   │   ├── fuzzy.rs                # nucleo integration
│   │   ├── ranking.rs              # Result ranking algorithm
│   │   └── providers/              # Search providers
│   │       ├── mod.rs
│   │       ├── apps.rs             # Application provider
│   │       ├── commands.rs         # System commands provider
│   │       └── files.rs            # Spotlight file provider
│   │
│   ├── indexer/                    # Application indexer
│   │   ├── mod.rs
│   │   ├── scanner.rs              # Filesystem scanning
│   │   ├── metadata.rs             # Info.plist parsing
│   │   ├── icons.rs                # Icon extraction
│   │   └── watcher.rs              # FS change detection
│   │
│   ├── commands/                   # System commands
│   │   ├── mod.rs
│   │   ├── system.rs               # Sleep, lock, restart, etc.
│   │   └── definitions.rs          # Command registry
│   │
│   ├── platform/                   # macOS integration
│   │   ├── mod.rs
│   │   ├── hotkey.rs               # Global hotkey registration
│   │   ├── accessibility.rs        # Permission checking/requesting
│   │   ├── spotlight.rs            # NSMetadataQuery wrapper
│   │   ├── launch.rs               # NSWorkspace app launching
│   │   └── appearance.rs           # System theme detection
│   │
│   ├── theme/                      # Theming system
│   │   ├── mod.rs
│   │   ├── catppuccin.rs           # Color palette definitions
│   │   ├── colors.rs               # Semantic color mapping
│   │   └── provider.rs             # GPUI theme integration
│   │
│   ├── storage/                    # Data persistence
│   │   ├── mod.rs
│   │   ├── database.rs             # rusqlite wrapper
│   │   └── usage.rs                # Usage frequency tracking
│   │
│   └── utils/                      # Shared utilities
│       ├── mod.rs
│       └── paths.rs                # Platform path helpers
│
├── resources/                      # Assets
│   ├── icons/
│   └── default_config.toml
│
├── tests/                          # Integration tests
│   ├── integration/
│   │   ├── search_test.rs
│   │   └── indexer_test.rs
│   └── fixtures/
│
└── benches/                        # Benchmarks
    ├── search_bench.rs
    └── render_bench.rs
```

### 2.3 Data Flow

```
User Types Query
       │
       ▼
┌──────────────┐
│  SearchBar   │────────────────┐
│  Component   │                │
└──────────────┘                │
       │                        │
       │ query string           │ on_change event
       ▼                        ▼
┌──────────────┐         ┌──────────────┐
│   Search     │         │    State     │
│   Engine     │         │   Update     │
└──────────────┘         └──────────────┘
       │
       │ dispatch to providers
       ▼
┌──────────────────────────────────────────┐
│             Search Providers              │
│  ┌────────┐  ┌──────────┐  ┌──────────┐  │
│  │  Apps  │  │ Commands │  │  Files   │  │
│  └────────┘  └──────────┘  └──────────┘  │
└──────────────────────────────────────────┘
       │
       │ raw results
       ▼
┌──────────────┐
│   Ranking    │ ← usage data from DB
│   Engine     │
└──────────────┘
       │
       │ sorted, grouped results
       ▼
┌──────────────┐
│   Results    │ → UI re-render via cx.notify()
│    List      │
└──────────────┘
       │
       │ user selection
       ▼
┌──────────────┐
│   Launch/    │
│   Execute    │
└──────────────┘
```

### 2.4 Concurrency Model

| Operation | Thread | Mechanism |
|-----------|--------|-----------|
| UI rendering | Main | GPUI run loop |
| Search execution | Background | `cx.spawn()` async task |
| App indexing | Background | Tokio task |
| FS watching | Background | Tokio + notify crate |
| Spotlight queries | Background | `tokio::task::spawn_blocking` |
| Database operations | Background | Async rusqlite |
| Hotkey handling | Main | CFRunLoop event source |

---

## 3. UI/UX Specification

### 3.1 Window Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  🔍  Search PhotonCast...                                     │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ─────────────────────────────────────────────────────────────────  │
│                                                                     │
│    Apps                                                      ⌘1-5   │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  🍎  Safari                                              ⌘1    │◀─ selected
│  │      Web Browser                                               │  │
│  ├───────────────────────────────────────────────────────────────┤  │
│  │  🔧  System Preferences                                  ⌘2    │  │
│  │      /System/Applications/System Preferences.app               │  │
│  ├───────────────────────────────────────────────────────────────┤  │
│  │  📝  Notes                                               ⌘3    │  │
│  │      Create notes and checklists                               │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│    Commands                                                         │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  🔒  Lock Screen                                         ⌘4    │  │
│  │      Lock your Mac                                             │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│    Files                                                            │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  📄  safari_bookmarks.html                               ⌘5    │  │
│  │      ~/Documents/safari_bookmarks.html                         │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Window Dimensions

| Property | Value | Notes |
|----------|-------|-------|
| Width | 680px | Fixed |
| Min Height | 72px | Search bar only |
| Max Height | 500px | ~10 results visible |
| Height | Dynamic | Based on result count |
| Border Radius | 12px | Rounded corners |
| Shadow | Large | Elevated appearance |
| Position | Centered-top | 20% from top of screen |
| Display | Cursor's display | Multi-monitor support |

### 3.3 Component Hierarchy

```rust
// Root window structure
LauncherWindow
├── SearchBar
│   ├── SearchIcon
│   └── TextInput
│       └── Placeholder ("Search PhotonCast...")
├── Divider (horizontal)
└── ResultsContainer (scrollable)
    ├── ResultGroup ("Apps")
    │   ├── GroupHeader
    │   └── ResultItem[] (apps)
    ├── ResultGroup ("Commands")
    │   ├── GroupHeader
    │   └── ResultItem[] (commands)
    └── ResultGroup ("Files")
        ├── GroupHeader
        └── ResultItem[] (files)
```

### 3.4 Component Specifications

#### SearchBar

```rust
pub struct SearchBar {
    query: String,
    focused: bool,
    placeholder: SharedString,
}

// Dimensions
const SEARCH_BAR_HEIGHT: Pixels = px(48.0);
const SEARCH_ICON_SIZE: Pixels = px(20.0);
const SEARCH_INPUT_FONT_SIZE: f32 = 16.0;

// Behavior
- Auto-focus on window show
- Clear on Escape
- Debounce input: 16ms (single frame at 60 FPS)
```

#### ResultItem

```rust
pub struct ResultItem {
    pub icon: Icon,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub shortcut: Option<String>,  // "⌘1", "⌘2", etc.
    pub result_type: ResultType,
    pub match_ranges: Vec<Range<usize>>,  // For highlighting
}

// Dimensions
const RESULT_ITEM_HEIGHT: Pixels = px(56.0);
const RESULT_ICON_SIZE: Pixels = px(32.0);
const RESULT_PADDING_X: Pixels = px(16.0);
const RESULT_PADDING_Y: Pixels = px(8.0);

// States
- Normal: default background
- Hover: surface_hover background
- Selected: surface_selected background with accent border
```

### 3.5 UI States

#### Loading State

```
┌─────────────────────────────────────────┐
│  🔍  Search PhotonCast...               │
├─────────────────────────────────────────┤
│                                         │
│     ◐  Indexing applications...         │
│        Found 142 of ~200 apps           │
│                                         │
└─────────────────────────────────────────┘
```

#### Empty State (No Query)

```
┌─────────────────────────────────────────┐
│  🔍  Search PhotonCast...               │
├─────────────────────────────────────────┤
│                                         │
│     Type to search apps, commands,      │
│     and files                           │
│                                         │
│     ↑↓ Navigate  ↵ Open  esc Close     │
│                                         │
└─────────────────────────────────────────┘
```

#### Empty State (No Results)

```
┌─────────────────────────────────────────┐
│  🔍  xyznonexistent                     │
├─────────────────────────────────────────┤
│                                         │
│     No results for "xyznonexistent"     │
│                                         │
│     Try a different search term         │
│                                         │
└─────────────────────────────────────────┘
```

#### Error State

```
┌─────────────────────────────────────────┐
│  🔍  Search PhotonCast...               │
├─────────────────────────────────────────┤
│                                         │
│  ⚠️  Indexing failed                    │
│                                         │
│     Unable to read /Applications        │
│     Check folder permissions            │
│                                         │
│     [ Retry ]  [ Open Folder ]          │
│                                         │
└─────────────────────────────────────────┘
```

### 3.6 Keyboard Shortcuts

| Shortcut | Action | Context |
|----------|--------|---------|
| `Cmd+Space` | Toggle launcher | Global (default) |
| `↓` / `Ctrl+N` | Select next result | In launcher |
| `↑` / `Ctrl+P` | Select previous result | In launcher |
| `Enter` | Activate selection | In launcher |
| `Escape` | Close launcher | In launcher |
| `⌘1` - `⌘9` | Quick select result 1-9 | In launcher |
| `⌘,` | Open preferences | In launcher |
| `Tab` | Cycle through groups | In launcher |

### 3.7 Animations

| Animation | Duration | Easing | Trigger |
|-----------|----------|--------|---------|
| Window appear | 150ms | ease-out | Hotkey activation |
| Window dismiss | 100ms | ease-in | Escape / blur |
| Selection change | 80ms | ease-in-out | Arrow keys |
| Result highlight | 60ms | linear | Mouse hover |
| Loading spinner | 1000ms | linear (loop) | During indexing |

**Reduce Motion Support:**
- Detect `NSWorkspace.accessibilityDisplayShouldReduceMotion`
- When enabled: instant transitions, no spring physics
- Respect PhotonCast settings override

```rust
pub fn animation_duration(base: Duration, cx: &App) -> Duration {
    let settings = cx.global::<Settings>();
    
    if settings.reduce_motion || cx.should_reduce_motion() {
        Duration::ZERO
    } else {
        base
    }
}
```

---

## 4. Core Systems

### 4.1 Search Engine

#### Architecture

```rust
pub struct SearchEngine {
    providers: Vec<Box<dyn SearchProvider>>,
    matcher: Matcher,
    ranker: ResultRanker,
}

pub trait SearchProvider: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, query: &str) -> Vec<RawSearchResult>;
    fn result_type(&self) -> ResultType;
}

pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub icon: IconSource,
    pub result_type: ResultType,
    pub score: u32,
    pub match_ranges: Vec<Range<usize>>,
    pub action: SearchAction,
}

pub enum SearchAction {
    LaunchApp { bundle_id: String, path: PathBuf },
    ExecuteCommand { command: SystemCommand },
    OpenFile { path: PathBuf },
    RevealInFinder { path: PathBuf },
}
```

#### nucleo Integration

```rust
use nucleo_matcher::{Matcher, Config};
use nucleo_matcher::pattern::{Pattern, CaseMatching, Normalization};

pub struct FuzzyMatcher {
    matcher: Matcher,
    config: MatcherConfig,
}

pub struct MatcherConfig {
    case_matching: CaseMatching,      // Smart (lowercase = insensitive)
    normalization: Normalization,     // Smart (unicode normalization)
    prefer_prefix: bool,              // Boost prefix matches
}

impl FuzzyMatcher {
    pub fn score(&mut self, query: &str, target: &str) -> Option<(u32, Vec<u32>)> {
        let pattern = Pattern::parse(query, self.config.case_matching, self.config.normalization);
        let mut indices = Vec::new();
        
        pattern.indices(target.chars(), &mut self.matcher, &mut indices)
            .map(|score| (score, indices))
    }
}
```

#### Search Flow

```rust
impl SearchEngine {
    pub async fn search(&self, query: &str) -> SearchResults {
        if query.is_empty() {
            return SearchResults::empty();
        }
        
        // 1. Collect results from all providers in parallel
        let raw_results: Vec<RawSearchResult> = futures::future::join_all(
            self.providers.iter().map(|p| p.search(query))
        ).await.into_iter().flatten().collect();
        
        // 2. Apply fuzzy matching and scoring
        let scored: Vec<ScoredResult> = raw_results
            .into_iter()
            .filter_map(|r| {
                self.matcher.score(query, &r.title)
                    .map(|(score, indices)| ScoredResult { result: r, score, indices })
            })
            .collect();
        
        // 3. Apply ranking (usage frequency, boosts)
        let ranked = self.ranker.rank(scored);
        
        // 4. Group by type
        SearchResults::grouped(ranked)
    }
}
```

### 4.2 Ranking Algorithm

#### Phase 1a: Pure Match Quality

```rust
pub fn rank_by_match_quality(results: &mut [ScoredResult]) {
    results.sort_by(|a, b| {
        // Primary: nucleo score (higher is better)
        b.score.cmp(&a.score)
    });
}
```

#### Phase 1b: Frecency Integration

```rust
pub struct FrecencyScore {
    pub frequency: u32,      // Total launch count
    pub recency: f64,        // Decay factor based on last use
    pub combined: f64,       // frequency * recency
}

impl FrecencyScore {
    pub fn calculate(launch_count: u32, last_launched: DateTime<Utc>) -> Self {
        let hours_since = (Utc::now() - last_launched).num_hours() as f64;
        
        // Decay: half-life of 72 hours
        let recency = 0.5_f64.powf(hours_since / 72.0);
        let combined = (launch_count as f64) * recency;
        
        Self { frequency: launch_count, recency, combined }
    }
}

pub fn rank_with_frecency(results: &mut [ScoredResult], usage_db: &UsageDatabase) {
    for result in results.iter_mut() {
        let frecency = usage_db.get_frecency(&result.id);
        result.final_score = (result.score as f64) + (frecency.combined * 10.0);
    }
    
    results.sort_by(|a, b| {
        b.final_score.partial_cmp(&a.final_score).unwrap_or(Ordering::Equal)
    });
}
```

#### Phase 1c: Boost Factors

```rust
pub struct BoostConfig {
    pub system_app_boost: f64,      // 1.2x for /System/Applications
    pub applications_boost: f64,    // 1.1x for /Applications
    pub exact_match_boost: f64,     // 2.0x for exact name match
    pub prefix_match_boost: f64,    // 1.5x for prefix match
}

impl Default for BoostConfig {
    fn default() -> Self {
        Self {
            system_app_boost: 1.2,
            applications_boost: 1.1,
            exact_match_boost: 2.0,
            prefix_match_boost: 1.5,
        }
    }
}

pub fn apply_boosts(result: &mut ScoredResult, query: &str, config: &BoostConfig) {
    // Path-based boosts
    if result.path.starts_with("/System/Applications") {
        result.final_score *= config.system_app_boost;
    } else if result.path.starts_with("/Applications") {
        result.final_score *= config.applications_boost;
    }
    
    // Match type boosts
    let title_lower = result.title.to_lowercase();
    let query_lower = query.to_lowercase();
    
    if title_lower == query_lower {
        result.final_score *= config.exact_match_boost;
    } else if title_lower.starts_with(&query_lower) {
        result.final_score *= config.prefix_match_boost;
    }
}
```

#### Tiebreaker Order

1. Usage count (higher wins)
2. Recency (more recent wins)
3. Alphabetical (A before Z)

### 4.3 App Indexer

#### Indexing Strategy

```rust
pub struct AppIndexer {
    index: Arc<RwLock<AppIndex>>,
    watcher: Option<RecommendedWatcher>,
}

pub struct AppIndex {
    pub apps: HashMap<String, IndexedApp>,
    pub last_full_scan: DateTime<Utc>,
    pub scan_duration: Duration,
}

pub struct IndexedApp {
    pub name: String,
    pub bundle_id: String,
    pub path: PathBuf,
    pub icon: Option<IconData>,
    pub keywords: Vec<String>,
    pub category: Option<String>,
    pub last_modified: DateTime<Utc>,
}
```

#### Scan Paths

```rust
const SCAN_PATHS: &[&str] = &[
    "/Applications",
    "/System/Applications",
    "~/Applications",
];

// Excluded patterns
const EXCLUDED_PATTERNS: &[&str] = &[
    "*.prefPane",           // Preference panes
    "*Uninstaller*.app",    // Uninstallers
    "*.app/Contents/*",     // Nested apps
];
```

#### Metadata Parsing

```rust
pub async fn parse_app_metadata(path: &Path) -> Result<IndexedApp> {
    let info_plist_path = path.join("Contents/Info.plist");
    let contents = tokio::fs::read(&info_plist_path).await
        .context("failed to read Info.plist")?;
    
    let plist: plist::Value = plist::from_bytes(&contents)
        .context("failed to parse Info.plist")?;
    let dict = plist.as_dictionary()
        .ok_or_else(|| anyhow!("Info.plist is not a dictionary"))?;
    
    // Extract name (try multiple keys)
    let name = dict.get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(|v| v.as_string())
        .map(String::from)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });
    
    // Extract bundle ID (required)
    let bundle_id = dict.get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(String::from)
        .ok_or_else(|| anyhow!("missing CFBundleIdentifier"))?;
    
    // Extract icon
    let icon = extract_icon(path, dict).await.ok();
    
    // Extract category
    let category = dict.get("LSApplicationCategoryType")
        .and_then(|v| v.as_string())
        .map(String::from);
    
    Ok(IndexedApp {
        name,
        bundle_id,
        path: path.to_path_buf(),
        icon,
        keywords: Vec::new(),
        category,
        last_modified: fs::metadata(path).await?.modified()?.into(),
    })
}
```

#### Background Re-indexing

```rust
impl AppIndexer {
    pub fn start_watcher(&mut self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                tx.send(event).ok();
            }
        })?;
        
        for path in SCAN_PATHS {
            let expanded = shellexpand::tilde(path);
            if Path::new(expanded.as_ref()).exists() {
                watcher.watch(Path::new(expanded.as_ref()), RecursiveMode::NonRecursive)?;
            }
        }
        
        self.watcher = Some(watcher);
        
        // Spawn handler task
        let index = Arc::clone(&self.index);
        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        // Debounce: wait 500ms for batch updates
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        
                        // Re-scan affected directory
                        if let Some(path) = event.paths.first() {
                            let mut idx = index.write().await;
                            idx.rescan_directory(path.parent().unwrap()).await.ok();
                        }
                    }
                    _ => {}
                }
            }
        });
        
        Ok(())
    }
}
```

### 4.4 Theme System

#### Catppuccin Implementation

```rust
use gpui::Hsla;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatppuccinFlavor {
    Latte,      // Light
    Frappe,     // Dark (low contrast)
    Macchiato,  // Dark (medium contrast)
    Mocha,      // Dark (high contrast)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentColor {
    Rosewater, Flamingo, Pink, Mauve,
    Red, Maroon, Peach, Yellow,
    Green, Teal, Sky, Sapphire,
    Blue, Lavender,
}

pub struct CatppuccinPalette {
    // Accent colors
    pub rosewater: Hsla,
    pub flamingo: Hsla,
    pub pink: Hsla,
    pub mauve: Hsla,
    pub red: Hsla,
    pub maroon: Hsla,
    pub peach: Hsla,
    pub yellow: Hsla,
    pub green: Hsla,
    pub teal: Hsla,
    pub sky: Hsla,
    pub sapphire: Hsla,
    pub blue: Hsla,
    pub lavender: Hsla,
    
    // Surface colors
    pub text: Hsla,
    pub subtext1: Hsla,
    pub subtext0: Hsla,
    pub overlay2: Hsla,
    pub overlay1: Hsla,
    pub overlay0: Hsla,
    pub surface2: Hsla,
    pub surface1: Hsla,
    pub surface0: Hsla,
    pub base: Hsla,
    pub mantle: Hsla,
    pub crust: Hsla,
}

impl CatppuccinPalette {
    pub fn mocha() -> Self {
        Self {
            // Mocha colors (see theming.md for full values)
            rosewater: hsla(10.0 / 360.0, 0.56, 0.91, 1.0),
            flamingo: hsla(0.0 / 360.0, 0.59, 0.88, 1.0),
            pink: hsla(316.0 / 360.0, 0.72, 0.86, 1.0),
            mauve: hsla(267.0 / 360.0, 0.84, 0.81, 1.0),
            red: hsla(343.0 / 360.0, 0.81, 0.75, 1.0),
            maroon: hsla(350.0 / 360.0, 0.65, 0.77, 1.0),
            peach: hsla(23.0 / 360.0, 0.92, 0.75, 1.0),
            yellow: hsla(41.0 / 360.0, 0.86, 0.83, 1.0),
            green: hsla(115.0 / 360.0, 0.54, 0.76, 1.0),
            teal: hsla(170.0 / 360.0, 0.57, 0.73, 1.0),
            sky: hsla(189.0 / 360.0, 0.71, 0.73, 1.0),
            sapphire: hsla(199.0 / 360.0, 0.76, 0.69, 1.0),
            blue: hsla(217.0 / 360.0, 0.92, 0.76, 1.0),
            lavender: hsla(232.0 / 360.0, 0.97, 0.85, 1.0),
            
            text: hsla(226.0 / 360.0, 0.64, 0.88, 1.0),
            subtext1: hsla(227.0 / 360.0, 0.35, 0.80, 1.0),
            subtext0: hsla(228.0 / 360.0, 0.24, 0.72, 1.0),
            overlay2: hsla(228.0 / 360.0, 0.17, 0.64, 1.0),
            overlay1: hsla(227.0 / 360.0, 0.13, 0.55, 1.0),
            overlay0: hsla(229.0 / 360.0, 0.11, 0.47, 1.0),
            surface2: hsla(228.0 / 360.0, 0.13, 0.40, 1.0),
            surface1: hsla(227.0 / 360.0, 0.15, 0.32, 1.0),
            surface0: hsla(230.0 / 360.0, 0.19, 0.23, 1.0),
            base: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
            mantle: hsla(240.0 / 360.0, 0.21, 0.12, 1.0),
            crust: hsla(240.0 / 360.0, 0.23, 0.09, 1.0),
        }
    }
    
    // Similar implementations for latte(), frappe(), macchiato()
}
```

#### Semantic Color Mapping

```rust
pub struct ThemeColors {
    // Backgrounds
    pub background: Hsla,
    pub background_elevated: Hsla,
    
    // Surfaces
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub surface_selected: Hsla,
    
    // Text
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub text_placeholder: Hsla,
    
    // Borders
    pub border: Hsla,
    pub border_focused: Hsla,
    
    // Accent
    pub accent: Hsla,
    pub accent_hover: Hsla,
    
    // Status
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    
    // Interactive
    pub selection: Hsla,
    pub hover: Hsla,
    pub focus_ring: Hsla,
    
    // Icons
    pub icon: Hsla,
    pub icon_accent: Hsla,
}

impl ThemeColors {
    pub fn from_palette(palette: &CatppuccinPalette, accent: AccentColor) -> Self {
        let accent_color = palette.get_accent(accent);
        
        Self {
            background: palette.base,
            background_elevated: palette.surface0,
            
            surface: palette.surface0,
            surface_hover: palette.surface1,
            surface_selected: accent_color.with_alpha(0.2),
            
            text: palette.text,
            text_secondary: palette.subtext1,
            text_muted: palette.subtext0,
            text_placeholder: palette.overlay1,
            
            border: palette.surface1,
            border_focused: accent_color,
            
            accent: accent_color,
            accent_hover: palette.lavender,
            
            success: palette.green,
            warning: palette.yellow,
            error: palette.red,
            
            selection: accent_color.with_alpha(0.2),
            hover: palette.surface1,
            focus_ring: accent_color.with_alpha(0.5),
            
            icon: palette.subtext0,
            icon_accent: accent_color,
        }
    }
}
```

#### System Theme Sync

```rust
use cocoa::appkit::{NSApp, NSAppearance};

pub fn detect_system_appearance() -> CatppuccinFlavor {
    unsafe {
        let app = NSApp();
        let appearance = app.effectiveAppearance();
        let name = appearance.name();
        
        if name.is_equal_to(NSAppearanceNameDarkAqua) {
            CatppuccinFlavor::Mocha
        } else {
            CatppuccinFlavor::Latte
        }
    }
}

pub fn observe_appearance_changes(cx: &mut App) {
    cx.observe_system_appearance(|cx| {
        let flavor = detect_system_appearance();
        let theme = cx.global::<PhotonTheme>();
        
        if theme.auto_sync && theme.flavor != flavor {
            cx.set_global(PhotonTheme::new(flavor, theme.accent));
            cx.refresh(); // Force full re-render
        }
    });
}
```

---

## 5. Platform Integration

### 5.1 Global Hotkey Registration

#### Implementation

```rust
use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventType, CGEventFlags};

pub struct HotkeyManager {
    registered: Option<HotkeyRegistration>,
    current_binding: HotkeyBinding,
}

#[derive(Clone, Debug)]
pub struct HotkeyBinding {
    pub key: KeyCode,
    pub modifiers: Modifiers,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            key: KeyCode::Space,
            modifiers: Modifiers::COMMAND,
        }
    }
}

#[derive(Debug)]
pub enum HotkeyError {
    PermissionDenied,
    ConflictDetected { conflicting_app: String },
    RegistrationFailed,
    InvalidBinding,
}

impl HotkeyManager {
    pub fn register(&mut self, binding: HotkeyBinding) -> Result<(), HotkeyError> {
        // 1. Check accessibility permission
        if !check_accessibility_permission() {
            return Err(HotkeyError::PermissionDenied);
        }
        
        // 2. Check for conflicts
        if let Some(conflict) = detect_hotkey_conflict(&binding) {
            return Err(HotkeyError::ConflictDetected {
                conflicting_app: conflict,
            });
        }
        
        // 3. Unregister existing
        self.unregister();
        
        // 4. Create event tap
        let tap = unsafe {
            CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsert,
                CGEventTapOptions::Default,
                vec![CGEventType::KeyDown, CGEventType::FlagsChanged],
                |proxy, event_type, event| {
                    // Check if event matches our binding
                    if matches_binding(event, &binding) {
                        // Trigger launcher toggle
                        HOTKEY_CALLBACK.with(|cb| {
                            if let Some(callback) = cb.borrow().as_ref() {
                                callback();
                            }
                        });
                        return None; // Consume event
                    }
                    Some(event)
                },
            )
        };
        
        // 5. Enable and add to run loop
        tap.enable();
        CFRunLoop::get_current().add_source(&tap.as_source(), kCFRunLoopDefaultMode);
        
        self.registered = Some(HotkeyRegistration { tap, binding: binding.clone() });
        self.current_binding = binding;
        
        Ok(())
    }
    
    pub fn unregister(&mut self) {
        if let Some(registration) = self.registered.take() {
            registration.tap.disable();
        }
    }
}
```

#### Conflict Detection

```rust
pub fn detect_hotkey_conflict(binding: &HotkeyBinding) -> Option<String> {
    // Check Spotlight (Cmd+Space)
    if binding.key == KeyCode::Space && binding.modifiers == Modifiers::COMMAND {
        if is_spotlight_shortcut_enabled() {
            return Some("Spotlight".to_string());
        }
    }
    
    // Check Siri (hold Cmd+Space)
    // Check other known launchers
    
    None
}

fn is_spotlight_shortcut_enabled() -> bool {
    // Read from ~/Library/Preferences/com.apple.symbolichotkeys.plist
    // Key 64 = Spotlight, check if enabled
    let path = dirs::home_dir()
        .unwrap()
        .join("Library/Preferences/com.apple.symbolichotkeys.plist");
    
    if let Ok(plist) = plist::from_file::<_, plist::Value>(&path) {
        if let Some(dict) = plist.as_dictionary() {
            if let Some(hotkeys) = dict.get("AppleSymbolicHotKeys") {
                if let Some(spotlight) = hotkeys.as_dictionary()?.get("64") {
                    return spotlight.as_dictionary()
                        .and_then(|d| d.get("enabled"))
                        .and_then(|v| v.as_boolean())
                        .unwrap_or(true);
                }
            }
        }
    }
    
    true // Assume enabled if can't read
}
```

#### Double-Tap Support

```rust
pub struct DoubleTapDetector {
    last_modifier_press: Option<Instant>,
    threshold: Duration,
    target_modifier: Modifiers,
}

impl DoubleTapDetector {
    pub fn new(modifier: Modifiers) -> Self {
        Self {
            last_modifier_press: None,
            threshold: Duration::from_millis(300),
            target_modifier: modifier,
        }
    }
    
    pub fn on_modifier_event(&mut self, modifiers: Modifiers, pressed: bool) -> bool {
        if modifiers != self.target_modifier {
            return false;
        }
        
        if pressed {
            if let Some(last) = self.last_modifier_press {
                if last.elapsed() < self.threshold {
                    self.last_modifier_press = None;
                    return true; // Double-tap detected!
                }
            }
            self.last_modifier_press = Some(Instant::now());
        }
        
        false
    }
}
```

### 5.2 Accessibility Permission Flow

#### Permission Checking

```rust
use accessibility_sys::{AXIsProcessTrusted, AXIsProcessTrustedWithOptions};
use core_foundation::dictionary::CFDictionary;
use core_foundation::boolean::CFBoolean;

pub enum PermissionStatus {
    Granted,
    Denied,
    Unknown,
}

pub fn check_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn request_accessibility_permission_with_prompt() {
    unsafe {
        let key = kAXTrustedCheckOptionPrompt;
        let options = CFDictionary::from_pairs(&[(key, CFBoolean::true_value())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}
```

#### Permission Dialog UI

```rust
pub struct PermissionDialog {
    status: PermissionStatus,
    checking: bool,
}

impl PermissionDialog {
    fn render_not_granted(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme(cx);
        
        v_flex()
            .gap_4()
            .p_6()
            .max_w(px(400.0))
            .child(
                Icon::new(IconName::Shield)
                    .size_12()
                    .color(theme.colors.accent)
            )
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .child("Accessibility Permission Required")
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.text_secondary)
                    .child("PhotonCast needs accessibility access to:")
            )
            .child(
                v_flex()
                    .gap_1()
                    .ml_4()
                    .child(bullet_item("Register global keyboard shortcuts"))
                    .child(bullet_item("Respond to hotkey activation"))
            )
            .child(
                h_flex()
                    .gap_3()
                    .mt_4()
                    .child(
                        Button::new("open_settings")
                            .label("Open System Settings")
                            .style(ButtonStyle::Primary)
                            .on_click(cx.listener(|_, _, _| {
                                open_accessibility_settings();
                            }))
                    )
                    .child(
                        Button::new("skip")
                            .label("Skip for Now")
                            .style(ButtonStyle::Ghost)
                            .on_click(cx.listener(|this, _, cx| {
                                this.dismiss(cx);
                            }))
                    )
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.colors.text_muted)
                    .mt_2()
                    .child("You can activate PhotonCast from the menu bar without this permission.")
            )
    }
}

fn open_accessibility_settings() {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .ok();
}
```

#### Real-time Permission Checking

```rust
impl PermissionDialog {
    pub fn start_permission_polling(&self, cx: &mut Context<Self>) {
        cx.spawn(|this, mut cx| async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                let granted = check_accessibility_permission();
                
                this.update(&mut cx, |this, cx| {
                    if granted && this.status != PermissionStatus::Granted {
                        this.status = PermissionStatus::Granted;
                        this.show_success_toast(cx);
                        cx.notify();
                    }
                }).ok();
                
                if granted {
                    break;
                }
            }
        }).detach();
    }
}
```

### 5.3 System Commands

#### Command Registry

```rust
#[derive(Debug, Clone)]
pub enum SystemCommand {
    Sleep,
    SleepDisplays,
    LockScreen,
    Restart,
    ShutDown,
    LogOut,
    EmptyTrash,
    ScreenSaver,
    ToggleAppearance,
}

impl SystemCommand {
    pub fn all() -> Vec<CommandInfo> {
        vec![
            CommandInfo {
                command: Self::Sleep,
                name: "Sleep",
                aliases: vec!["sleep", "suspend"],
                description: "Put Mac to sleep",
                icon: IconName::Moon,
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::LockScreen,
                name: "Lock Screen",
                aliases: vec!["lock"],
                description: "Lock your Mac",
                icon: IconName::Lock,
                requires_confirmation: false,
            },
            CommandInfo {
                command: Self::Restart,
                name: "Restart",
                aliases: vec!["restart", "reboot"],
                description: "Restart your Mac",
                icon: IconName::RotateCcw,
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::ShutDown,
                name: "Shut Down",
                aliases: vec!["shutdown", "power off"],
                description: "Shut down your Mac",
                icon: IconName::Power,
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::LogOut,
                name: "Log Out",
                aliases: vec!["logout", "sign out"],
                description: "Log out current user",
                icon: IconName::LogOut,
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::EmptyTrash,
                name: "Empty Trash",
                aliases: vec!["empty trash", "clear trash"],
                description: "Empty the Trash",
                icon: IconName::Trash,
                requires_confirmation: true,
            },
            CommandInfo {
                command: Self::ScreenSaver,
                name: "Screen Saver",
                aliases: vec!["screensaver"],
                description: "Start screen saver",
                icon: IconName::Monitor,
                requires_confirmation: false,
            },
        ]
    }
}
```

#### Command Execution

```rust
impl SystemCommand {
    pub fn execute(&self) -> Result<()> {
        match self {
            Self::Sleep => {
                Command::new("pmset")
                    .args(["sleepnow"])
                    .spawn()
                    .context("failed to execute sleep command")?;
            }
            
            Self::SleepDisplays => {
                Command::new("pmset")
                    .args(["displaysleepnow"])
                    .spawn()
                    .context("failed to sleep displays")?;
            }
            
            Self::LockScreen => {
                // Use Quartz Display Services
                unsafe {
                    CGSession::lock();
                }
            }
            
            Self::Restart => {
                let script = r#"
                    tell application "System Events"
                        restart
                    end tell
                "#;
                run_applescript(script)?;
            }
            
            Self::ShutDown => {
                let script = r#"
                    tell application "System Events"
                        shut down
                    end tell
                "#;
                run_applescript(script)?;
            }
            
            Self::LogOut => {
                let script = r#"
                    tell application "System Events"
                        log out
                    end tell
                "#;
                run_applescript(script)?;
            }
            
            Self::EmptyTrash => {
                let script = r#"
                    tell application "Finder"
                        empty trash
                    end tell
                "#;
                run_applescript(script)?;
            }
            
            Self::ScreenSaver => {
                Command::new("open")
                    .args(["-a", "ScreenSaverEngine"])
                    .spawn()
                    .context("failed to start screen saver")?;
            }
            
            Self::ToggleAppearance => {
                let script = r#"
                    tell application "System Events"
                        tell appearance preferences
                            set dark mode to not dark mode
                        end tell
                    end tell
                "#;
                run_applescript(script)?;
            }
        }
        
        Ok(())
    }
}

fn run_applescript(script: &str) -> Result<()> {
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .context("failed to run AppleScript")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("AppleScript error: {}", stderr);
    }
    
    Ok(())
}
```

### 5.4 Spotlight Integration

#### File Search Provider

```rust
use objc2_foundation::{NSMetadataQuery, NSMetadataQueryDidFinishGatheringNotification};

pub struct SpotlightProvider {
    query: Option<NSMetadataQuery>,
}

impl SpotlightProvider {
    pub async fn search(&self, query_text: &str, limit: usize) -> Result<Vec<FileResult>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        
        tokio::task::spawn_blocking(move || {
            let query = unsafe {
                let q = NSMetadataQuery::new();
                
                // Build predicate for name search
                let predicate = NSPredicate::predicateWithFormat(
                    &NSString::from_str(&format!(
                        "kMDItemDisplayName LIKE[cd] '*{}*'",
                        query_text.replace("'", "\\'")
                    ))
                );
                q.setPredicate(&predicate);
                
                // Set result limit
                q.setResultLimit(limit as i32);
                
                // Search scope
                let scopes = NSArray::from_vec(vec![
                    NSMetadataQueryUserHomeScope,
                    NSMetadataQueryLocalComputerScope,
                ]);
                q.setSearchScopes(&scopes);
                
                // Start query
                q.startQuery();
                
                // Wait for completion (with timeout)
                let run_loop = CFRunLoop::get_current();
                run_loop.run_in_mode(kCFRunLoopDefaultMode, Duration::from_millis(500), false);
                
                q.stopQuery();
                q
            };
            
            // Extract results
            let results = unsafe {
                let count = query.resultCount();
                (0..count)
                    .filter_map(|i| {
                        let item = query.resultAtIndex(i)?;
                        FileResult::from_metadata_item(&item)
                    })
                    .collect()
            };
            
            tx.send(results).ok();
        });
        
        rx.await.map_err(|_| anyhow!("Spotlight query failed"))
    }
}

#[derive(Debug)]
pub struct FileResult {
    pub path: PathBuf,
    pub name: String,
    pub kind: FileKind,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

impl FileResult {
    fn from_metadata_item(item: &NSMetadataItem) -> Option<Self> {
        unsafe {
            let path = item.valueForAttribute(NSMetadataItemPathKey)?
                .downcast::<NSString>()?
                .to_string();
            
            let name = item.valueForAttribute(NSMetadataItemDisplayNameKey)?
                .downcast::<NSString>()?
                .to_string();
            
            Some(Self {
                path: PathBuf::from(path),
                name,
                kind: FileKind::from_path(&path),
                size: 0,
                modified: Utc::now(),
            })
        }
    }
}
```

---

## 6. Data Models

### 6.1 Core Types

```rust
//! Core domain types for PhotonCast

use std::path::PathBuf;
use std::ops::Range;
use chrono::{DateTime, Utc};

/// Unique identifier for indexed applications
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

/// Information about an indexed application
#[derive(Debug, Clone)]
pub struct Application {
    pub bundle_id: AppBundleId,
    pub name: String,
    pub path: PathBuf,
    pub icon: Option<IconData>,
    pub category: Option<AppCategory>,
    pub last_modified: DateTime<Utc>,
}

/// Application category (from Info.plist)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCategory {
    DeveloperTools,
    Entertainment,
    Finance,
    Graphics,
    Healthcare,
    Lifestyle,
    Medical,
    Music,
    News,
    Photography,
    Productivity,
    SocialNetworking,
    Sports,
    Travel,
    Utilities,
    Video,
    Weather,
    Other(String),
}

/// Icon data that can be rendered
#[derive(Debug, Clone)]
pub enum IconData {
    Cached { path: PathBuf },
    Inline { data: Vec<u8>, format: ImageFormat },
    System { name: String },
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Icns,
    Png,
    Jpeg,
}
```

### 6.2 Search Types

```rust
/// A search result that can be displayed and activated
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: SearchResultId,
    pub title: String,
    pub subtitle: String,
    pub icon: IconSource,
    pub result_type: ResultType,
    pub score: f64,
    pub match_indices: Vec<usize>,
    pub action: SearchAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchResultId(String);

/// Source of an icon
#[derive(Debug, Clone)]
pub enum IconSource {
    AppIcon { bundle_id: AppBundleId },
    SystemIcon { name: String },
    FileIcon { path: PathBuf },
    Emoji { char: char },
}

/// Type of search result for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultType {
    Application,
    SystemCommand,
    File,
    Folder,
}

impl ResultType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Application => "Apps",
            Self::SystemCommand => "Commands",
            Self::File => "Files",
            Self::Folder => "Folders",
        }
    }
    
    pub fn priority(&self) -> u8 {
        match self {
            Self::Application => 0,
            Self::SystemCommand => 1,
            Self::File => 2,
            Self::Folder => 3,
        }
    }
}

/// Action to perform when result is activated
#[derive(Debug, Clone)]
pub enum SearchAction {
    LaunchApp {
        bundle_id: AppBundleId,
        path: PathBuf,
    },
    ExecuteCommand {
        command: SystemCommand,
    },
    OpenFile {
        path: PathBuf,
    },
    RevealInFinder {
        path: PathBuf,
    },
}

/// Grouped search results for display
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub groups: Vec<ResultGroup>,
    pub total_count: usize,
    pub query: String,
    pub search_time: Duration,
}

#[derive(Debug, Clone)]
pub struct ResultGroup {
    pub result_type: ResultType,
    pub results: Vec<SearchResult>,
}
```

### 6.3 Configuration Types

```rust
/// User configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    
    #[serde(default)]
    pub appearance: AppearanceConfig,
    
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    
    #[serde(default)]
    pub launch_at_login: bool,
    
    #[serde(default)]
    pub show_in_dock: bool,
    
    #[serde(default)]
    pub show_in_menu_bar: bool,
}

fn default_max_results() -> usize { 10 }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HotkeyConfig {
    #[serde(default = "default_hotkey")]
    pub key: String,
    
    #[serde(default = "default_modifiers")]
    pub modifiers: Vec<String>,
    
    #[serde(default)]
    pub double_tap_modifier: Option<String>,
}

fn default_hotkey() -> String { "Space".to_string() }
fn default_modifiers() -> Vec<String> { vec!["Command".to_string()] }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppearanceConfig {
    #[serde(default)]
    pub theme: ThemeSetting,
    
    #[serde(default = "default_accent")]
    pub accent_color: String,
    
    #[serde(default)]
    pub reduce_motion: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
    Latte,
    Frappe,
    Macchiato,
    Mocha,
}

fn default_accent() -> String { "mauve".to_string() }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    #[serde(default)]
    pub include_system_apps: bool,
    
    #[serde(default = "default_file_limit")]
    pub file_result_limit: usize,
    
    #[serde(default)]
    pub excluded_apps: Vec<String>,
}

fn default_file_limit() -> usize { 5 }
```

### 6.4 Database Schema

```sql
-- Usage tracking for frecency ranking
CREATE TABLE IF NOT EXISTS app_usage (
    bundle_id TEXT PRIMARY KEY,
    launch_count INTEGER NOT NULL DEFAULT 0,
    last_launched_at INTEGER NOT NULL,  -- Unix timestamp
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_app_usage_last_launched ON app_usage(last_launched_at DESC);

-- Command usage
CREATE TABLE IF NOT EXISTS command_usage (
    command_id TEXT PRIMARY KEY,
    execute_count INTEGER NOT NULL DEFAULT 0,
    last_executed_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

-- File access tracking
CREATE TABLE IF NOT EXISTS file_usage (
    path TEXT PRIMARY KEY,
    open_count INTEGER NOT NULL DEFAULT 0,
    last_opened_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

-- App index cache
CREATE TABLE IF NOT EXISTS app_cache (
    bundle_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    icon_path TEXT,
    category TEXT,
    last_modified INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL
);
```

---

## 7. Error Handling

### 7.1 Error Types

```rust
use thiserror::Error;

/// Errors from the search subsystem
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
        source: anyhow::Error,
    },
    
    #[error("search timeout after {duration:?}")]
    Timeout { duration: Duration },
}

/// Errors from the app indexer
#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("failed to scan directory '{path}': {source}")]
    ScanError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    
    #[error("failed to parse Info.plist for '{app}': {reason}")]
    PlistError {
        app: String,
        reason: String,
    },
    
    #[error("icon extraction failed for '{app}': {source}")]
    IconError {
        app: String,
        #[source]
        source: anyhow::Error,
    },
    
    #[error("database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
}

/// Errors from hotkey registration
#[derive(Error, Debug)]
pub enum HotkeyError {
    #[error("accessibility permission required")]
    PermissionDenied,
    
    #[error("hotkey conflict with '{app}'")]
    ConflictDetected { app: String },
    
    #[error("failed to register hotkey: {reason}")]
    RegistrationFailed { reason: String },
    
    #[error("invalid key combination")]
    InvalidBinding,
}

/// Errors from system command execution
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("command '{command}' failed: {reason}")]
    ExecutionFailed {
        command: String,
        reason: String,
    },
    
    #[error("authorization required for '{command}'")]
    AuthorizationRequired { command: String },
    
    #[error("command not available on this system")]
    NotAvailable,
}

/// Errors from app launching
#[derive(Error, Debug)]
pub enum LaunchError {
    #[error("application not found: {bundle_id}")]
    NotFound { bundle_id: String },
    
    #[error("failed to launch '{app}': {reason}")]
    LaunchFailed {
        app: String,
        reason: String,
    },
    
    #[error("application is damaged and can't be opened")]
    Damaged { app: String },
}

/// Errors from configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("invalid config format: {0}")]
    ParseError(#[from] toml::de::Error),
    
    #[error("config validation failed: {reason}")]
    ValidationError { reason: String },
}
```

### 7.2 User-Facing Messages

```rust
impl SearchError {
    pub fn user_message(&self) -> String {
        match self {
            Self::IndexNotReady => "Still indexing applications. Please wait...".into(),
            Self::InvalidQuery { .. } => "Invalid search query".into(),
            Self::ProviderError { provider, .. } => {
                format!("{} search temporarily unavailable", provider)
            }
            Self::Timeout { .. } => "Search took too long. Try a shorter query.".into(),
        }
    }
    
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::IndexNotReady | Self::ProviderError { .. })
    }
}

impl HotkeyError {
    pub fn user_message(&self) -> String {
        match self {
            Self::PermissionDenied => {
                "PhotonCast needs accessibility permission for global hotkeys.".into()
            }
            Self::ConflictDetected { app } => {
                format!("Hotkey is already used by {}. Please choose another.", app)
            }
            Self::RegistrationFailed { .. } => {
                "Failed to register hotkey. Try a different combination.".into()
            }
            Self::InvalidBinding => "Invalid key combination.".into(),
        }
    }
    
    pub fn action_hint(&self) -> Option<String> {
        match self {
            Self::PermissionDenied => Some("Open System Settings".into()),
            Self::ConflictDetected { .. } => Some("Change Hotkey".into()),
            _ => None,
        }
    }
}

impl LaunchError {
    pub fn user_message(&self) -> String {
        match self {
            Self::NotFound { bundle_id } => {
                format!("Application '{}' is no longer installed", bundle_id)
            }
            Self::LaunchFailed { app, reason } => {
                format!("Couldn't open {}: {}", app, reason)
            }
            Self::Damaged { app } => {
                format!("{} is damaged and can't be opened. Try reinstalling.", app)
            }
        }
    }
}
```

### 7.3 Error Recovery

```rust
pub struct ErrorRecovery;

impl ErrorRecovery {
    pub fn handle_search_error(error: &SearchError, cx: &mut Context<Launcher>) {
        match error {
            SearchError::IndexNotReady => {
                // Show loading state, will auto-resolve
                cx.update_state(|s| s.show_indexing_progress = true);
            }
            SearchError::ProviderError { provider, .. } => {
                // Log and continue with other providers
                tracing::warn!("Provider {} failed, continuing without", provider);
            }
            SearchError::Timeout { .. } => {
                // Show partial results if available
                cx.show_toast("Search timed out. Showing partial results.");
            }
            _ => {
                cx.show_error_state(&error.user_message());
            }
        }
    }
    
    pub fn handle_launch_error(error: &LaunchError, cx: &mut Context<Launcher>) {
        match error {
            LaunchError::NotFound { bundle_id } => {
                // Remove from index, notify user
                cx.indexer.remove_app(bundle_id);
                cx.show_toast(&error.user_message());
            }
            LaunchError::Damaged { app } => {
                // Offer to reveal in Finder
                cx.show_error_dialog(
                    &error.user_message(),
                    &[("Reveal in Finder", Action::RevealInFinder)],
                );
            }
            _ => {
                cx.show_toast(&error.user_message());
            }
        }
    }
}
```

---

## 8. Performance

### 8.1 Performance Targets

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Cold start | < 100ms | Time from process start to first frame |
| Hotkey response | < 50ms | Time from key press to window visible |
| Search latency | < 30ms | Time from keystroke to results displayed |
| Memory (idle) | < 50MB | Activity Monitor RSS |
| Memory (active) | < 100MB | With large search results |
| UI rendering | 120 FPS | Consistent during animations |
| Index time (full) | < 2s | For ~200 applications |
| Index time (incremental) | < 100ms | Single app update |

### 8.2 Benchmarks

```rust
// benches/search_bench.rs
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_fuzzy_search(c: &mut Criterion) {
    let matcher = FuzzyMatcher::new(MatcherConfig::default());
    let apps = load_test_apps(200);
    
    c.bench_function("fuzzy_search_200_apps", |b| {
        b.iter(|| {
            let query = black_box("safari");
            apps.iter()
                .filter_map(|app| matcher.score(query, &app.name))
                .collect::<Vec<_>>()
        })
    });
}

fn bench_ranking(c: &mut Criterion) {
    let ranker = ResultRanker::new(BoostConfig::default());
    let results = generate_test_results(100);
    
    c.bench_function("rank_100_results", |b| {
        b.iter(|| {
            let mut results = black_box(results.clone());
            ranker.rank(&mut results);
        })
    });
}

fn bench_render_results(c: &mut Criterion) {
    // GPUI render benchmark
    c.bench_function("render_10_results", |b| {
        b.iter(|| {
            let results = black_box(generate_test_results(10));
            // Measure render time
        })
    });
}

criterion_group!(benches, bench_fuzzy_search, bench_ranking, bench_render_results);
criterion_main!(benches);
```

### 8.3 Optimization Strategies

#### Search Optimization

```rust
// Pre-compute search data
pub struct SearchIndex {
    // Pre-lowercase for case-insensitive matching
    apps_lowercase: Vec<(usize, String)>,
    
    // Pre-sort by frequency for early termination
    apps_by_frequency: Vec<usize>,
}

impl SearchIndex {
    pub fn search(&self, query: &str, max_results: usize) -> Vec<ScoredResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::with_capacity(max_results);
        
        // Search most frequent apps first
        for &idx in &self.apps_by_frequency {
            let (_, ref name_lower) = self.apps_lowercase[idx];
            
            if let Some(score) = self.matcher.score(&query_lower, name_lower) {
                results.push(ScoredResult { idx, score });
                
                // Early termination if we have enough high-quality matches
                if results.len() >= max_results * 2 {
                    break;
                }
            }
        }
        
        // Sort and truncate
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(max_results);
        results
    }
}
```

#### Rendering Optimization

```rust
// Virtual scrolling for large result lists
impl ResultsList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible_range = self.calculate_visible_range();
        let item_height = RESULT_ITEM_HEIGHT;
        
        div()
            .h_full()
            .overflow_y_scroll()
            // Spacer for items above viewport
            .child(div().h(px(visible_range.start as f32 * item_height.0)))
            // Only render visible items
            .children(
                self.results[visible_range.clone()]
                    .iter()
                    .enumerate()
                    .map(|(i, result)| {
                        let global_idx = visible_range.start + i;
                        ResultItem::new(result.clone())
                            .selected(global_idx == self.selected_index)
                    })
            )
            // Spacer for items below viewport
            .child(div().h(px(
                (self.results.len() - visible_range.end) as f32 * item_height.0
            )))
    }
}
```

#### Memory Optimization

```rust
// Lazy icon loading
pub struct LazyIcon {
    path: PathBuf,
    data: OnceCell<IconData>,
}

impl LazyIcon {
    pub fn get(&self) -> Option<&IconData> {
        self.data.get_or_try_init(|| {
            load_icon_from_path(&self.path)
        }).ok()
    }
}

// Bounded caches
pub struct IconCache {
    cache: LruCache<AppBundleId, IconData>,
}

impl IconCache {
    pub fn new() -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
        }
    }
}
```

### 8.4 Performance Monitoring

```rust
use tracing::{info, span, Level};

pub fn measure_search<F, R>(query: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let span = span!(Level::INFO, "search", query = %query);
    let _enter = span.enter();
    
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    
    info!(elapsed_ms = elapsed.as_millis(), "search completed");
    
    if elapsed > Duration::from_millis(30) {
        tracing::warn!(
            elapsed_ms = elapsed.as_millis(),
            "search exceeded 30ms target"
        );
    }
    
    result
}
```

---

## 9. Testing Strategy

### 9.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Fuzzy matching tests
    #[test]
    fn test_fuzzy_exact_match() {
        let matcher = FuzzyMatcher::new(MatcherConfig::default());
        let (score, indices) = matcher.score("Safari", "Safari").unwrap();
        
        assert!(score > 100);
        assert_eq!(indices, vec![0, 1, 2, 3, 4, 5]);
    }
    
    #[test]
    fn test_fuzzy_prefix_match() {
        let matcher = FuzzyMatcher::new(MatcherConfig::default());
        let result = matcher.score("saf", "Safari");
        
        assert!(result.is_some());
        let (_, indices) = result.unwrap();
        assert_eq!(indices, vec![0, 1, 2]);
    }
    
    #[test]
    fn test_fuzzy_case_insensitive() {
        let matcher = FuzzyMatcher::new(MatcherConfig::default());
        
        let lower = matcher.score("safari", "Safari");
        let upper = matcher.score("SAFARI", "Safari");
        
        assert!(lower.is_some());
        assert!(upper.is_some());
    }
    
    #[test]
    fn test_fuzzy_no_match() {
        let matcher = FuzzyMatcher::new(MatcherConfig::default());
        let result = matcher.score("xyz", "Safari");
        
        assert!(result.is_none());
    }
    
    // Ranking tests
    #[test]
    fn test_frecency_calculation() {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::hours(24);
        
        let recent = FrecencyScore::calculate(10, now);
        let older = FrecencyScore::calculate(10, one_day_ago);
        
        assert!(recent.combined > older.combined);
    }
    
    #[test]
    fn test_boost_application_path() {
        let config = BoostConfig::default();
        
        let mut system_app = ScoredResult {
            path: PathBuf::from("/System/Applications/Safari.app"),
            final_score: 100.0,
            ..Default::default()
        };
        
        let mut user_app = ScoredResult {
            path: PathBuf::from("/Applications/Firefox.app"),
            final_score: 100.0,
            ..Default::default()
        };
        
        apply_boosts(&mut system_app, "s", &config);
        apply_boosts(&mut user_app, "f", &config);
        
        assert!(system_app.final_score > user_app.final_score);
    }
    
    // Config tests
    #[test]
    fn test_config_defaults() {
        let config: Config = toml::from_str("").unwrap();
        
        assert_eq!(config.general.max_results, 10);
        assert!(!config.general.launch_at_login);
    }
    
    #[test]
    fn test_config_hotkey_parsing() {
        let toml = r#"
            [hotkey]
            key = "Space"
            modifiers = ["Command", "Option"]
        "#;
        
        let config: Config = toml::from_str(toml).unwrap();
        
        assert_eq!(config.hotkey.key, "Space");
        assert_eq!(config.hotkey.modifiers, vec!["Command", "Option"]);
    }
}
```

### 9.2 Integration Tests

```rust
// tests/integration/search_test.rs

#[tokio::test]
async fn test_search_engine_integration() {
    let engine = SearchEngine::new_test();
    
    // Add test applications
    engine.add_test_app(Application {
        bundle_id: AppBundleId::new("com.apple.Safari"),
        name: "Safari".into(),
        path: PathBuf::from("/Applications/Safari.app"),
        ..Default::default()
    });
    
    engine.add_test_app(Application {
        bundle_id: AppBundleId::new("com.apple.finder"),
        name: "Finder".into(),
        path: PathBuf::from("/System/Library/CoreServices/Finder.app"),
        ..Default::default()
    });
    
    // Test search
    let results = engine.search("saf").await.unwrap();
    
    assert_eq!(results.groups.len(), 1);
    assert_eq!(results.groups[0].result_type, ResultType::Application);
    assert_eq!(results.groups[0].results[0].title, "Safari");
}

#[tokio::test]
async fn test_app_indexer_integration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let apps_dir = temp_dir.path().join("Applications");
    fs::create_dir(&apps_dir).unwrap();
    
    // Create fake app bundle
    create_test_app_bundle(&apps_dir, "TestApp", "com.test.app").unwrap();
    
    // Index
    let indexer = AppIndexer::new();
    let apps = indexer.scan_directory(&apps_dir).await.unwrap();
    
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "TestApp");
    assert_eq!(apps[0].bundle_id.as_str(), "com.test.app");
}

fn create_test_app_bundle(dir: &Path, name: &str, bundle_id: &str) -> Result<()> {
    let app_dir = dir.join(format!("{}.app", name));
    let contents_dir = app_dir.join("Contents");
    fs::create_dir_all(&contents_dir)?;
    
    let info_plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
            <key>CFBundleName</key>
            <string>{}</string>
            <key>CFBundleIdentifier</key>
            <string>{}</string>
        </dict>
        </plist>"#, name, bundle_id);
    
    fs::write(contents_dir.join("Info.plist"), info_plist)?;
    Ok(())
}
```

### 9.3 Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzzy_search_never_panics(query in ".*", target in ".*") {
        let matcher = FuzzyMatcher::new(MatcherConfig::default());
        let _ = matcher.score(&query, &target);
    }
    
    #[test]
    fn ranking_is_deterministic(
        results in prop::collection::vec(arbitrary_result(), 0..100)
    ) {
        let ranker = ResultRanker::new(BoostConfig::default());
        
        let mut results1 = results.clone();
        let mut results2 = results.clone();
        
        ranker.rank(&mut results1);
        ranker.rank(&mut results2);
        
        for (r1, r2) in results1.iter().zip(results2.iter()) {
            prop_assert_eq!(r1.id, r2.id);
        }
    }
    
    #[test]
    fn config_roundtrip(config in arbitrary_config()) {
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        
        prop_assert_eq!(config.general.max_results, parsed.general.max_results);
    }
}

fn arbitrary_result() -> impl Strategy<Value = ScoredResult> {
    (any::<String>(), 0u32..1000u32).prop_map(|(id, score)| {
        ScoredResult {
            id: SearchResultId(id),
            score: score as f64,
            ..Default::default()
        }
    })
}
```

### 9.4 Test Coverage Targets

| Component | Target | Priority |
|-----------|--------|----------|
| Fuzzy matching | 90% | High |
| Ranking algorithm | 90% | High |
| Config parsing | 80% | Medium |
| Error handling | 80% | Medium |
| UI components | 60% | Low |
| Platform FFI | 40% | Low |

---

## 10. Implementation Phases

### 10.1 Sprint 1: Core UI Framework (Weeks 1-4)

#### Week 1-2: Foundation

| Task | Description | Estimate |
|------|-------------|----------|
| Project setup | Cargo workspace, CI/CD, linting | 2d |
| GPUI integration | Window creation, event loop | 3d |
| Theme system | Catppuccin palette, color mapping | 2d |
| Basic window | Empty launcher window with styling | 2d |

**Deliverables:**
- [ ] Empty launcher window appears
- [ ] Theme colors applied correctly
- [ ] Window can be shown/hidden

#### Week 3-4: Core Components

| Task | Description | Estimate |
|------|-------------|----------|
| SearchBar component | Input, placeholder, focus handling | 3d |
| ResultsList component | Scrollable list container | 2d |
| ResultItem component | Icon, title, subtitle, selection | 3d |
| Keyboard navigation | Arrow keys, enter, escape | 2d |
| Empty/loading states | UI feedback components | 1d |

**Deliverables:**
- [ ] Search bar accepts input
- [ ] Results list renders items
- [ ] Keyboard navigation works
- [ ] 120 FPS rendering verified

**Sprint 1 Acceptance Criteria:**
- Window appears/disappears in under 50ms
- Keyboard navigation is intuitive (↑↓, Enter, Esc)
- UI renders at consistent 120 FPS
- All 4 Catppuccin themes working

---

### 10.2 Sprint 2: App Launcher (Weeks 5-8)

#### Week 5-6: Indexing

| Task | Description | Estimate |
|------|-------------|----------|
| App scanner | Discover apps in standard paths | 2d |
| Metadata parser | Info.plist extraction | 2d |
| Icon extraction | Load app icons from bundles | 2d |
| Database setup | rusqlite schema, migrations | 1d |
| FS watcher | Detect app install/removal | 2d |

**Deliverables:**
- [ ] Apps indexed from all standard paths
- [ ] Metadata (name, bundle ID) extracted
- [ ] Icons loading correctly
- [ ] Index persisted to database

#### Week 7-8: Search & Launch

| Task | Description | Estimate |
|------|-------------|----------|
| nucleo integration | Fuzzy matching setup | 2d |
| Search provider (apps) | Query apps with scoring | 2d |
| Ranking algorithm | Match quality + frecency | 3d |
| App launching | NSWorkspace integration | 2d |
| Usage tracking | Record launches for frecency | 1d |

**Deliverables:**
- [ ] Apps searchable by name
- [ ] Fuzzy matching working
- [ ] Apps launch correctly
- [ ] Usage affects ranking

**Sprint 2 Acceptance Criteria:**
- Index 200+ apps in under 2 seconds
- Search results appear in under 30ms
- Correct app launches on Enter
- Frecency ranking improves with use

---

### 10.3 Sprint 3: Global Hotkey & System (Weeks 9-12)

#### Week 9-10: Hotkey & Permissions

| Task | Description | Estimate |
|------|-------------|----------|
| Accessibility check | Permission status detection | 1d |
| Permission dialog | In-app guided flow | 2d |
| Hotkey registration | CGEventTap integration | 3d |
| Conflict detection | Check for Spotlight, etc. | 2d |
| Hotkey customization | Settings UI, validation | 2d |

**Deliverables:**
- [ ] Permission status detected
- [ ] Guided permission flow
- [ ] Global hotkey registers
- [ ] Conflicts detected and warned

#### Week 11-12: System Commands & Files

| Task | Description | Estimate |
|------|-------------|----------|
| System commands | Sleep, lock, restart, etc. | 3d |
| Command provider | Search integration | 1d |
| Spotlight integration | NSMetadataQuery wrapper | 3d |
| File provider | Search files, display results | 2d |
| Result grouping | Apps, Commands, Files sections | 2d |

**Deliverables:**
- [ ] System commands execute
- [ ] Commands searchable
- [ ] Files searchable via Spotlight
- [ ] Results grouped by type

**Sprint 3 Acceptance Criteria:**
- Hotkey responds within 50ms
- System commands execute correctly
- File search returns results in under 100ms
- Clear permission flow with recovery

---

### 10.4 Release Checklist (v0.1.0-alpha)

#### Functional

- [ ] App search with fuzzy matching
- [ ] App launching via NSWorkspace
- [ ] Global hotkey (Cmd+Space default)
- [ ] Keyboard navigation (↑↓, Enter, Esc)
- [ ] System commands (sleep, lock, restart, shutdown, logout, empty trash, screen saver)
- [ ] File search via Spotlight
- [ ] Result grouping (Apps, Commands, Files)
- [ ] Quick select (⌘1-9)
- [ ] Frecency-based ranking

#### Non-Functional

- [ ] Cold start < 100ms
- [ ] Hotkey response < 50ms
- [ ] Search latency < 30ms
- [ ] Memory < 50MB idle
- [ ] 120 FPS rendering
- [ ] All 4 Catppuccin themes
- [ ] System theme sync
- [ ] Reduce motion support

#### Quality

- [ ] Unit test coverage > 60%
- [ ] Integration tests passing
- [ ] No clippy warnings
- [ ] Documentation complete
- [ ] Error messages user-friendly

---

## 11. Open Questions / Risks

### 11.1 Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| GPUI API instability | High | Medium | Pin versions, maintain local patches |
| CGEventTap reliability | High | Low | Fallback to Carbon events |
| Spotlight query performance | Medium | Medium | Implement caching, result limits |
| Icon extraction complexity | Low | Medium | Graceful fallback to generic icon |
| Memory with large icon cache | Medium | Low | LRU cache with size limits |

### 11.2 Open Questions

| Question | Status | Owner | Notes |
|----------|--------|-------|-------|
| GPUI window positioning API | Research needed | - | May need custom implementation |
| Double-tap modifier timing | Needs testing | - | 300ms threshold tentative |
| Spotlight result freshness | Research needed | - | How quickly does index update? |
| AppleScript vs direct API | Decision needed | - | Security vs simplicity tradeoff |

### 11.3 Known Limitations (Phase 1)

| Limitation | Workaround | Future Solution |
|------------|------------|-----------------|
| No clipboard history | Use macOS built-in | Phase 2 |
| No window management | Use Rectangle/Magnet | Phase 2 |
| No extension support | Built-in commands only | Phase 3 |
| No custom themes | 4 Catppuccin flavors | Phase 4+ |
| Single-user only | - | Not planned |
| macOS only | - | Not planned |

### 11.4 Dependencies

| Dependency | Version | Risk | Notes |
|------------|---------|------|-------|
| gpui | 0.1.x | Medium | Not yet 1.0, API may change |
| gpui-component | 0.1.x | Medium | Relies on gpui stability |
| nucleo | 0.5.x | Low | Stable, used by Helix |
| rusqlite | 0.31.x | Low | Very stable |
| tokio | 1.x | Low | Very stable |
| objc2 | 0.5.x | Low | Stable Rust bindings |

---

## Appendix A: Reference Links

- [GPUI Documentation](https://gpui.rs)
- [gpui-component Library](https://github.com/longbridge/gpui-component)
- [Loungy Launcher](https://github.com/MatthiasGrandl/Loungy) - Reference implementation
- [Zed Editor](https://github.com/zed-industries/zed) - GPUI in production
- [nucleo Fuzzy Matcher](https://github.com/helix-editor/nucleo)
- [Catppuccin Theme](https://github.com/catppuccin/catppuccin)
- [Raycast](https://raycast.com) - Visual reference

---

## Appendix B: Configuration Example

```toml
# ~/.config/photoncast/config.toml

[general]
max_results = 10
launch_at_login = true
show_in_dock = false
show_in_menu_bar = true

[hotkey]
key = "Space"
modifiers = ["Command"]
# Alternative: double-tap Command
# double_tap_modifier = "Command"

[appearance]
theme = "system"  # latte, frappe, macchiato, mocha, system
accent_color = "mauve"
reduce_motion = false

[search]
include_system_apps = true
file_result_limit = 5
excluded_apps = [
    "com.apple.installer",
    "com.apple.ScriptEditor2",
]
```

---

*Document generated: 2026-01-15*  
*PhotonCast Phase 1 MVP Specification v1.0*
