# Architecture

## Overview

PhotonCast is a macOS launcher application (~101K lines of Rust) built with GPUI (Zed's GPU-accelerated UI framework). It provides instant search across applications, commands, files, and extensions with sub-100ms response times.

## Crate Dependency Graph

```
photoncast (binary)
├── photoncast-core          # Foundation: search, extensions, platform, storage
│   ├── photoncast-theme     # Catppuccin color scheme
│   └── photoncast-extension-api  # ABI-stable extension trait
├── photoncast-apps          # App management
├── photoncast-calculator    # Calculator/converter
├── photoncast-calendar      # EventKit integration
├── photoncast-clipboard     # Clipboard history
├── photoncast-quicklinks    # Quick links
├── photoncast-timer         # Sleep timer
└── photoncast-window        # Window management

Extension dylibs:
├── photoncast-ext-github             ─┐
├── photoncast-ext-screenshots         ├── depend on photoncast-extension-api
└── photoncast-ext-system-preferences ─┘
```

**Dependency rules:**
- `photoncast-core` is the foundation; most crates depend on it
- Feature crates (calculator, clipboard, etc.) are independent of each other
- Extension crates depend only on `photoncast-extension-api` (ABI boundary)
- The main binary (`photoncast`) ties everything together

## Data Flow

### Search Pipeline

```
User keystroke
  → LauncherWindow::on_query_change()
    → SearchEngine::search(query)
      → Parallel providers: Apps, Commands, Files, Extensions, Calendar, Timer, QuickLinks, Windows
      → Each provider returns scored results
    → Merge + deduplicate + rank (frecency boost)
    → Top-K selection (max 20 results)
  → UI re-render with results
```

### Extension Loading Pipeline

```
App startup / extension directory change
  → ExtensionManager::discover_extensions()
    → Scan ~/Library/Application Support/PhotonCast/extensions/
    → Parse extension.toml manifests
    → resolve_entry_path() with path traversal prevention
    → verify_code_signature() (skipped in dev_mode)
    → libloading::Library::new() to load dylib
    → abi_stable root module extraction
    → Extension::on_load() callback
    → Register commands as search providers
```

### Theme System

```
App startup / macOS appearance change
  → PhotonTheme::new(flavor, accent) set as GPUI global
  → Each view reads cx.try_global::<PhotonTheme>()
  → Colors derived from Catppuccin palette
  → cx.observe_window_appearance() watches for OS dark/light changes
```

## Key Architectural Decisions

1. **GPUI for rendering** — Zed's GPU-accelerated UI framework. Provides immediate-mode rendering, reactive state management, and native macOS integration. Pinned to a specific Zed commit for stability.

2. **ABI-stable extension API** — Extensions are compiled as separate dylibs using `abi_stable` crate. This allows extensions to be built with different Rust compiler versions. The `Extension` trait defines the plugin interface.

3. **SQLite for persistence** — `rusqlite` (bundled) for usage tracking (frecency), clipboard history, quick links, and timer state. Each subsystem manages its own database file.

4. **Catppuccin theming** — Four flavors (Latte, Frappe, Macchiato, Mocha) with automatic light/dark mode sync via macOS appearance observation.

5. **macOS-only** — Deep platform integration: CGEventTap for global hotkeys, Spotlight/NSMetadataQuery for file indexing, EventKit for calendar, accessibility APIs for window management, Keychain for encryption keys.

## Key Modules (photoncast-core)

| Module | Purpose |
|--------|---------|
| `search/` | Search engine, result providers, fuzzy matching (nucleo), ranking |
| `search/spotlight/` | NSMetadataQuery integration, live file index, prefetch |
| `extensions/` | Extension lifecycle: discovery, loading, signing, runtime, permissions, storage |
| `indexer/` | App scanning (/Applications, Homebrew), file watching, metadata extraction |
| `platform/` | macOS APIs: hotkeys, accessibility, file actions, appearance, launch, menu bar |
| `storage/` | SQLite database abstraction, usage tracking (frecency) |
| `commands/` | System commands (sleep, restart, empty trash, etc.) |
| `custom_commands/` | User-defined shell commands with placeholder expansion |
| `app/` | App configuration, keybindings, integration layer |
| `ui/` | Shared UI components (search bar, result items, animations) |

## Module Structure (photoncast binary)

| Module | Purpose |
|--------|---------|
| `launcher/` | Main launcher window (split into 9 sub-modules: render, search, actions, animation, calculator, calendar, indexing, uninstall) |
| `file_search_view/` | File search UI (split into 5 sub-modules: render, filter, browsing, helpers) |
| `extension_views/` | Extension view rendering (list, detail, grid, form, preview, navigation, actions) |
| `preferences_window/` | Settings UI (split into 10+ sub-modules by section) |
| `icon_cache.rs` | App icon caching with filesystem-backed LRU |
| `main.rs` | App initialization, event loop, window management |
