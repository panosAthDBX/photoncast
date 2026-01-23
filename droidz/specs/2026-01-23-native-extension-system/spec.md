# PhotonCast Sprint 6 - Native Extension System Specification

> **Status:** Draft  
> **Date:** 2026-01-23  
> **Target Version:** v1.2.0  
> **Priority:** High

---

## Table of Contents

1. [Overview](#1-overview)
2. [Goals & Non-Goals](#2-goals--non-goals)
3. [User Stories](#3-user-stories)
4. [Architecture](#4-architecture)
5. [Extension Manifest (TOML)](#5-extension-manifest-toml)
6. [Extension Lifecycle](#6-extension-lifecycle)
7. [Extension API Surface](#7-extension-api-surface)
8. [Custom Commands](#8-custom-commands)
9. [Reference Extensions](#9-reference-extensions)
10. [Security Considerations](#10-security-considerations)
11. [Performance Requirements](#11-performance-requirements)
12. [Error Handling Strategy](#12-error-handling-strategy)
13. [Testing & Verification](#13-testing--verification)
14. [Phase 3: Raycast Extension Compatibility](#14-phase-3-raycast-extension-compatibility)

---

## 1. Overview

Sprint 6 introduces a **native Rust extension system** for PhotonCast. Extensions are compiled Rust dynamic libraries loaded by the app at runtime and can:

- Contribute search results to the main launcher search.
- Expose commands with their own UI (lists, detail views, forms).
- Use scoped storage and preferences.
- Support fast dev workflows via hot-reload.

This sprint also adds **Custom Commands** (user-defined shell commands) and ships **first-party reference extensions** to validate the API and UX.

---

## 2. Goals & Non-Goals

### 2.1 Goals

1. **Native extension architecture** (no Node.js sidecar).
2. **Manifest-driven metadata** in TOML.
3. **Stable extension API** for search + UI + storage.
4. **Hot-reload** for development (watch & reload in place).
5. **Performance**: load/activate extension in <50ms.
6. **Custom Commands**: shell execution with output capture and notifications.
7. **Reference extensions**: GitHub repos browser, System Preferences shortcuts, Color Picker.

### 2.2 Non-Goals

- Raycast extension compatibility (Node sidecar). *(Phase 3)*
- Extension store or remote marketplace.
- True sandboxing/isolation (extensions run in-process).
- Cross-platform extension support (macOS only).

---

## 3. User Stories

### 3.1 Extension Developers

- **US-1:** As a Rust developer, I want a simple manifest and API so I can build extensions quickly.
- **US-2:** As a developer, I want hot-reload so I can iterate without restarting PhotonCast.
- **US-3:** As a developer, I want to expose search results and commands with minimal boilerplate.

### 3.2 End Users

- **US-4:** As a user, I want to enable/disable extensions safely.
- **US-5:** As a user, I want extension commands to appear in the launcher like built-in commands.
- **US-6:** As a user, I want custom shell commands that capture output and show errors.

---

## 4. Architecture

### 4.1 High-Level Components

```
┌──────────────────────────────────────────────────────────┐
│                      PhotonCast App                      │
├──────────────────────────────────────────────────────────┤
│  Extension Manager                                       │
│  ┌──────────────────────┐  ┌──────────────────────────┐  │
│  │ Extension Registry   │  │ Extension Loader         │  │
│  │ - manifests          │  │ - libloading             │  │
│  │ - enable/disable     │  │ - ABI version check      │  │
│  └──────────────────────┘  └──────────────────────────┘  │
│                                                          │
│  Search Engine (SearchProvider trait)                    │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Built-ins  |  Extension Providers  |  Custom Cmds   │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  Extension Host API                                      │
│  ┌────────────────────────────────────────────────────┐  │
│  │ UI (List/Detail/Form) | Storage | Clipboard | Toast │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### 4.2 Extension Packaging Layout

```
~/Library/Application Support/PhotonCast/extensions/<extension_id>/
├── extension.toml
├── lib/
│   └── lib<extension_id>.dylib
├── assets/
│   ├── icon.png
│   └── ...
└── README.md (optional)
```

### 4.3 Extension Types

1. **Search Provider Extension**
   - Contributes results to global search via `SearchProvider`.
2. **Command Extension**
   - Exposes commands that open a view or run actions.
3. **Hybrid**
   - Both search provider + commands.

---

## 5. Extension Manifest (TOML)

### 5.1 File Name

`extension.toml`

### 5.2 Required Schema (v1)

```toml
schema_version = 1

[extension]
id = "com.photoncast.github"
name = "GitHub Repos"
version = "0.1.0"
description = "Search and open GitHub repositories"
author = "PhotonCast"
license = "MIT"
homepage = "https://github.com/photoncast/extensions"
min_photoncast_version = "1.2.0"
api_version = 1

[entry]
kind = "cdylib"
path = "lib/libcom_photoncast_github.dylib"

[permissions]
network = true
clipboard = true
notifications = true
filesystem = ["~/Documents", "~/Downloads"]

[[commands]]
id = "search_repos"
name = "Search Repositories"
mode = "view"        # view | search | no-view
keywords = ["github", "repo", "repositories"]
icon = "github"      # SF Symbol, emoji, or asset path
subtitle = "GitHub"

[[preferences]]
name = "api_token"
type = "secret"      # string | number | boolean | secret | select | file | directory
required = true
title = "GitHub API Token"
description = "Used for GitHub API requests"
```

### 5.3 Validation Rules

- `extension.id` must be reverse-DNS and globally unique.
- `version` must be SemVer.
- `api_version` must be supported by the host.
- `entry.path` must exist and be a `.dylib`.
- Command IDs must be unique within the extension.

---

## 6. Extension Lifecycle

### 6.1 States

- **Discovered** → manifest parsed
- **Loaded** → dynamic library loaded
- **Active** → providers/commands registered
- **Disabled** → user-disabled, not executed
- **Failed** → load or activation error
- **Unloaded** → library handle dropped

### 6.2 State Transitions

```
Discovered → Loaded → Active
Active → Disabled → Unloaded
Loaded → Failed
Active → Failed → Disabled
```

### 6.3 Lifecycle Hooks

```rust
pub trait Extension {
    fn manifest(&self) -> ExtensionManifest;
    fn activate(&mut self, ctx: ExtensionContext) -> Result<()>;
    fn deactivate(&mut self) -> Result<()>;
    fn search_provider(&self) -> Option<Box<dyn ExtensionSearchProvider>>;
    fn commands(&self) -> Vec<ExtensionCommand>;
}
```

### 6.4 Hot-Reload (Development)

- Enabled when `extensions.dev_mode = true` or `PHOTONCAST_DEV_EXTENSIONS=1`.
- Watch `extension.toml` and `.dylib` for changes.
- Reload by copying dylib to a **versioned cache path** to bypass OS loader caching:
  - `cache/extensions/<id>/<timestamp>.dylib`
- On reload: `deactivate()` → drop providers → unload dylib → load new dylib → `activate()`.
- If reload fails, extension is marked **Failed** and disabled until next change.

---

## 7. Extension API Surface

### 7.1 API Crate

Provide a dedicated crate for extension authors:

```
crates/photoncast-extension-api
```

This crate defines all stable types and traits. It is shared by host and extension to ensure ABI compatibility (use `abi_stable`).

### 7.2 Core Types

```rust
pub struct ExtensionContext {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub preferences: PreferenceStore,
    pub storage: ExtensionStorage,
    pub host: ExtensionHost,
    pub runtime: ExtensionRuntime,
}

pub struct ExtensionHost {
    pub fn show_toast(&self, toast: Toast);
    pub fn open_url(&self, url: &str);
    pub fn open_file(&self, path: &Path);
    pub fn reveal_in_finder(&self, path: &Path);
    pub fn copy_to_clipboard(&self, text: &str);
    pub fn read_clipboard(&self) -> Option<String>;
    pub fn selected_text(&self) -> Option<String>;
}
```

### 7.3 Search Provider API

```rust
pub trait ExtensionSearchProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn search(&self, query: &str, max_results: usize) -> Vec<ExtensionSearchItem>;
}

pub struct ExtensionSearchItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: IconSource,
    pub score: f64,
    pub actions: Vec<ExtensionAction>,
}
```

### 7.4 Command API

```rust
pub struct ExtensionCommand {
    pub id: String,
    pub name: String,
    pub mode: CommandMode,
    pub keywords: Vec<String>,
    pub handler: Box<dyn CommandHandler>,
}

pub enum CommandMode {
    Search,   // Integrates into global search
    View,     // Opens a custom view
    NoView,   // Executes background action
}
```

### 7.5 UI API (Declarative View Schema)

Extensions define UI using a **declarative schema**. The host renders all UI using GPUI, ensuring consistent styling, animations, and keyboard navigation across all extensions.

#### 7.5.1 View Types

```rust
pub enum ExtensionView {
    /// List view with items, sections, and search
    List(ListView),
    /// Detail view with markdown content and metadata
    Detail(DetailView),
    /// Form view for user input
    Form(FormView),
    /// Grid view for visual items (images, icons)
    Grid(GridView),
}
```

#### 7.5.2 List View (Primary View Type)

```rust
pub struct ListView {
    pub title: String,
    pub search_bar: Option<SearchBarConfig>,
    pub sections: Vec<ListSection>,
    pub empty_state: Option<EmptyState>,
    /// Enable split-view with preview panel
    pub show_preview: bool,
}

pub struct SearchBarConfig {
    pub placeholder: String,
    pub throttle_ms: u32,  // Default: 100ms
}

pub struct ListSection {
    pub title: Option<String>,
    pub items: Vec<ListItem>,
}

pub struct ListItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: IconSource,
    /// Accessories shown on right side (tags, dates, shortcuts)
    pub accessories: Vec<Accessory>,
    /// Actions available via Cmd+K or right-click
    pub actions: Vec<Action>,
    /// Preview content for split-view (rendered in right panel)
    pub preview: Option<Preview>,
    /// Keyboard shortcut hint (e.g., "⏎" for default action)
    pub shortcut: Option<String>,
}

pub enum Accessory {
    Text(String),
    Tag { text: String, color: TagColor },
    Date(SystemTime),
    Icon(IconSource),
}

pub enum Preview {
    /// Markdown content
    Markdown(String),
    /// Image from path or URL
    Image { source: String, alt: String },
    /// Metadata key-value pairs
    Metadata(Vec<(String, String)>),
}

pub struct EmptyState {
    pub icon: Option<IconSource>,
    pub title: String,
    pub description: Option<String>,
    pub actions: Vec<Action>,  // e.g., "Create New", "Open Settings"
}
```

#### 7.5.3 Detail View

```rust
pub struct DetailView {
    pub title: String,
    pub markdown: String,
    pub metadata: Vec<MetadataItem>,
    pub actions: Vec<Action>,
}

pub struct MetadataItem {
    pub label: String,
    pub value: MetadataValue,
}

pub enum MetadataValue {
    Text(String),
    Link { text: String, url: String },
    Date(SystemTime),
    Tag { text: String, color: TagColor },
}
```

#### 7.5.4 Form View

```rust
pub struct FormView {
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<FormField>,
    pub submit: SubmitButton,
}

pub struct FormField {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub placeholder: Option<String>,
    pub default_value: Option<String>,
    pub validation: Option<Validation>,
}

pub enum FieldType {
    TextField,
    TextArea { rows: u32 },
    Password,
    Number { min: Option<f64>, max: Option<f64> },
    Checkbox,
    Dropdown { options: Vec<DropdownOption> },
    FilePicker { allowed_extensions: Vec<String> },
    DirectoryPicker,
    DatePicker,
}

pub struct SubmitButton {
    pub label: String,
    pub shortcut: Option<String>,  // e.g., "⌘⏎"
}
```

#### 7.5.5 Grid View

```rust
pub struct GridView {
    pub title: String,
    pub columns: u32,  // 2-6
    pub items: Vec<GridItem>,
    pub empty_state: Option<EmptyState>,
}

pub struct GridItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub image: ImageSource,
    pub actions: Vec<Action>,
}

pub enum ImageSource {
    Path(PathBuf),
    Url(String),
    Base64 { data: String, mime_type: String },
    SfSymbol(String),
}
```

### 7.6 Action System (Cmd+K Menu)

Actions are the primary way extensions expose functionality. They appear in:
- **Actions Menu** (Cmd+K) - contextual actions for selected item
- **Item Actions** - inline on list/grid items
- **Global Actions** - available from any view

#### 7.6.1 Action Definition

```rust
pub struct Action {
    pub id: String,
    pub title: String,
    pub icon: Option<IconSource>,
    pub shortcut: Option<Shortcut>,
    pub style: ActionStyle,
    pub handler: ActionHandler,
}

pub enum ActionStyle {
    Default,
    Destructive,  // Red text, confirmation required
    Primary,      // Highlighted as main action
}

pub enum ActionHandler {
    /// Run a callback function
    Callback(ActionCallback),
    /// Open a URL
    OpenUrl(String),
    /// Open a file
    OpenFile(PathBuf),
    /// Copy text to clipboard
    CopyToClipboard(String),
    /// Push a new view onto the navigation stack
    PushView(Box<ExtensionView>),
    /// Submit form data
    SubmitForm,
}

pub struct Shortcut {
    pub key: String,           // e.g., "c", "enter", "backspace"
    pub modifiers: Modifiers,  // cmd, shift, alt, ctrl
}

pub struct Modifiers {
    pub cmd: bool,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}
```

#### 7.6.2 Standard Actions (Built-in)

Extensions can use these pre-built actions for consistency:

```rust
impl Action {
    /// Copy text to clipboard - shows "Copied!" toast
    pub fn copy(text: impl Into<String>) -> Self;
    
    /// Open URL in default browser
    pub fn open_url(url: impl Into<String>) -> Self;
    
    /// Open file with default app
    pub fn open_file(path: impl Into<PathBuf>) -> Self;
    
    /// Reveal file in Finder
    pub fn reveal_in_finder(path: impl Into<PathBuf>) -> Self;
    
    /// Show Quick Look preview
    pub fn quick_look(path: impl Into<PathBuf>) -> Self;
    
    /// Delete with confirmation
    pub fn delete_with_confirmation(
        title: impl Into<String>,
        handler: ActionCallback,
    ) -> Self;
}
```

#### 7.6.3 Action Registration

Actions are registered per-item and rendered by the host in the Cmd+K menu:

```rust
// In ListItem
ListItem {
    id: "repo-123".into(),
    title: "photoncast".into(),
    actions: vec![
        Action::open_url("https://github.com/user/photoncast")
            .with_title("Open in Browser")
            .with_shortcut(Shortcut::cmd("o")),
        Action::copy("git@github.com:user/photoncast.git")
            .with_title("Copy SSH URL")
            .with_shortcut(Shortcut::cmd_shift("c")),
        Action::callback(|ctx| { /* custom logic */ })
            .with_title("Star Repository")
            .with_icon(IconSource::SfSymbol("star")),
    ],
    ..Default::default()
}
```

### 7.7 Navigation API

Extensions can push/pop views to create multi-screen flows:

```rust
pub trait Navigation {
    /// Push a new view onto the stack
    fn push(&self, view: ExtensionView);
    
    /// Pop current view and return to previous
    fn pop(&self);
    
    /// Replace current view
    fn replace(&self, view: ExtensionView);
    
    /// Pop to root view
    fn pop_to_root(&self);
}

// Available via ExtensionContext
impl ExtensionContext {
    pub fn navigation(&self) -> &dyn Navigation;
}
```

**Navigation behavior:**
- Back: `Escape` or `Cmd+[` pops the view stack
- Views animate in/out with consistent transitions
- Navigation state is preserved per-extension

### 7.8 View Updates & Async Data

Extensions can update views asynchronously:

```rust
pub trait ViewHandle: Send + Sync {
    /// Update the entire view
    fn update(&self, view: ExtensionView);
    
    /// Update just the items in a list (efficient for search)
    fn update_items(&self, items: Vec<ListItem>);
    
    /// Show loading state
    fn set_loading(&self, loading: bool);
    
    /// Show error state
    fn set_error(&self, error: Option<String>);
}

// Example: async search
impl ExtensionSearchProvider for GitHubProvider {
    fn search(&self, query: &str, ctx: SearchContext) -> SearchHandle {
        let handle = ctx.view_handle();
        handle.set_loading(true);
        
        // Spawn async task
        ctx.runtime.spawn(async move {
            match fetch_repos(query).await {
                Ok(repos) => {
                    let items = repos.into_iter().map(repo_to_item).collect();
                    handle.update_items(items);
                }
                Err(e) => handle.set_error(Some(e.to_string())),
            }
            handle.set_loading(false);
        });
        
        SearchHandle::Pending
    }
}
```

### 7.9 Design System & Consistency

To ensure uniform UX across all extensions, the host enforces:

#### 7.9.1 Rendering Constraints

| Element | Constraint |
|---------|------------|
| Icons | 16x16, 24x24, or 32x32 - auto-scaled |
| Thumbnails | Max 64x64 in lists, 256x256 in preview |
| Title text | Single line, truncated with ellipsis |
| Subtitle text | Single line, muted color |
| Action icons | 16x16, SF Symbols preferred |
| Grid columns | 2-6, auto-calculated item size |

#### 7.9.2 Color System

Extensions **cannot specify custom colors**. They use semantic tokens:

```rust
pub enum TagColor {
    Default,   // Gray
    Blue,
    Green,
    Yellow,
    Orange,
    Red,
    Purple,
    Pink,
}

// Host maps these to theme-appropriate colors
```

#### 7.9.3 Typography

All text uses the host's typography system:
- **Title**: SF Pro Text, 14pt, Medium
- **Subtitle**: SF Pro Text, 12pt, Regular, 60% opacity
- **Accessory**: SF Pro Text, 11pt, Regular
- **Markdown**: Rendered with host styles (headings, code, links)

#### 7.9.4 Keyboard Navigation

The host provides consistent keyboard navigation:

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate items |
| `⏎` | Activate default action |
| `⌘K` | Open actions menu |
| `⌘Y` | Quick Look (if preview available) |
| `⎋` | Close menu / Pop view / Dismiss |
| `Tab` | Next section / field |
| `⌘1-9` | Quick select item by position |

#### 7.9.5 Animation

All animations are handled by the host:
- List item hover: subtle background highlight
- Selection: animated highlight
- View transitions: slide left/right (150ms)
- Loading: standardized spinner
- Toast: slide up, auto-dismiss

### 7.10 Storage API

- Backed by SQLite.
- Namespaced by `extension.id`.
- Supports `get`, `set`, `delete`, `list`.
- **Secrets** stored in macOS Keychain.

---

## 8. Custom Commands

### 8.1 Definition

Custom commands are user-defined shell commands that appear in the launcher and execute with optional arguments.

### 8.2 Data Model (SQLite)

```sql
CREATE TABLE custom_commands (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    alias TEXT,
    command TEXT NOT NULL,
    args TEXT,
    cwd TEXT,
    shell TEXT DEFAULT "/bin/zsh",
    env_json TEXT,
    capture_output BOOLEAN DEFAULT 1,
    show_notifications BOOLEAN DEFAULT 1,
    timeout_ms INTEGER DEFAULT 10000,
    requires_confirmation BOOLEAN DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    last_run_at TEXT,
    run_count INTEGER DEFAULT 0,
    enabled BOOLEAN DEFAULT 1
);
```

### 8.3 Placeholders

Support placeholder interpolation before execution:

| Placeholder | Description |
|-------------|-------------|
| `{query}` | Text after command name in launcher |
| `{selection}` | Selected text from frontmost app |
| `{clipboard}` | Clipboard text |
| `{env:VAR}` | Environment variable `VAR` |

### 8.4 Execution

- Executed via `tokio::process::Command`.
- If `shell` is set, execute: `shell -lc "<command> <args>"`.
- Output capture (stdout + stderr) is stored and shown in a detail view.
- Non-zero exit codes show a failure toast with exit code.

### 8.5 Output UI

- Success: `Toast::Success` + optional output preview.
- Failure: `Toast::Failure` + action: "View Output".
- Output view shows:
  - Command
  - Exit status
  - Captured stdout/stderr (truncated at 64KB)

---

## 9. Reference Extensions

### 9.1 GitHub Repositories Browser

**Manifest:**
- `id = com.photoncast.github`
- Permissions: `network`, `clipboard`
- Preferences: `api_token` (secret), `default_org` (string)

**Command:**
- `Search Repositories` (mode: view)

**Behavior:**
- User types a query → list of repositories.
- Each list item shows repo name, description, stars, language.
- Actions:
  - Open in browser
  - Copy HTTPS URL
  - Copy SSH URL
  - Open Issues
  - Open Pull Requests

### 9.2 System Preferences Shortcuts

**Manifest:**
- `id = com.photoncast.settings`
- No permissions required

**Command:**
- `Open System Settings` (mode: view)

**Behavior:**
- List of system settings panes (Wi‑Fi, Bluetooth, Privacy, Sound, etc.).
- Each item opens `x-apple.systempreferences:` deep link.

### 9.3 Screenshot Browser

**Manifest:**
- `id = com.photoncast.screenshots`
- Permissions: `clipboard`, `filesystem`
- Preferences: `screenshots_folder` (directory, default: `~/Desktop`)

**Command:**
- `Browse Screenshots` (mode: view)

**Behavior:**
- Lists screenshots from configured folder (default: Desktop).
- Sorted by date (newest first).
- Search bar filters by filename.
- Each list item shows:
  - Thumbnail preview
  - Filename
  - Date taken
  - File size
- Large preview panel on right side (split view).
- Actions:
  - Copy to Clipboard (default on Enter)
  - Open in Preview
  - Reveal in Finder
  - Delete (with confirmation)
  - Quick Look (Cmd+Y)

**Preferences UI:**
- Configure monitored folder path
- Option to include subfolders
- Filter by file extension (png, jpg, etc.)

---

## 10. Security Considerations

1. **No sandboxing in v1.** Extensions run in-process with full app privileges.
2. **Explicit user trust.** Enabling an extension shows permissions summary.
3. **Restricted install locations.** Only load from the application support directory or explicit dev paths.
4. **Manifest permissions are advisory.** Used for UI prompts and warnings.
5. **Custom commands** may run arbitrary shell scripts; require confirmation if flagged.

---

## 11. Performance Requirements

| Requirement | Target |
|-------------|--------|
| Extension load time | < 50ms |
| Search provider response | < 20ms per query |
| Hot reload cycle | < 250ms |
| View update latency | < 16ms |

**Implementation notes:**
- Cache manifests in memory.
- Lazily load extensions on first use unless `auto_load=true`.
- Limit provider results to `SearchConfig.max_results_per_provider`.

---

## 12. Error Handling Strategy

- Use typed errors (`ExtensionError`, `ManifestError`).
- Any load/activate failure marks extension as **Failed** and disables it.
- Repeated failures are rate-limited and surfaced as notifications.
- Search provider errors are isolated (do not crash search engine).
- Custom command errors display:
  - Exit code
  - stderr output (if captured)
  - Action: "View Output"

---

## 13. Testing & Verification

### 13.1 Unit Tests

- Manifest parsing & validation.
- Version/ABI compatibility checks.
- Placeholder expansion for custom commands.

### 13.2 Integration Tests

- Load/activate/deactivate/unload lifecycle.
- Search integration with a mock extension provider.
- Hot-reload with dylib replacement.

### 13.3 Performance Tests

- Extension load benchmark (<50ms).
- Provider search benchmark (<20ms).
- Custom command execution latency & output capture.

---

## 14. Phase 3: Raycast Extension Compatibility

> **Status:** Planning  
> **Target Version:** v1.4.0 (future)  
> **Priority:** Medium

This section documents the architectural decisions and implementation plan for running Raycast extensions in PhotonCast. The goal is to enable users to install and run existing Raycast extensions with minimal or no modification.

### 14.1 Raycast Extension Architecture Overview

Raycast extensions are built with:

| Component | Technology |
|-----------|------------|
| Language | TypeScript/JavaScript |
| UI Framework | React (custom reconciler) |
| Runtime | Node.js (managed by Raycast) |
| API Package | `@raycast/api` (npm) |
| Manifest | `package.json` with Raycast-specific fields |
| Build Tool | esbuild (embedded in `ray` CLI) |

**Key insight**: Raycast bundles all npm dependencies at build time using esbuild. The published extension is a single self-contained JS file, not raw source + node_modules.

### 14.2 IPC Protocol

Raycast uses **JSON-RPC** over stdio streams for communication between the Swift host and Node.js process.

**Why JSON-RPC (and why we'll use it too):**
- Proven at scale (Raycast handles complex extensions like Jira, GitLab)
- Easy to debug (plain text)
- Well-supported by OSS libraries
- Fast enough for UI updates
- Matches Raycast exactly (simplifies compatibility)

**Message flow:**
```
Extension (Node Worker) ←→ Node Parent Process ←→ PhotonCast (via stdio/JSON-RPC)
```

**UI updates use JSON Patch (RFC 6902)** for efficient incremental rendering:
```json
{ "op": "replace", "path": "/sections/0/items/2/title", "value": "Updated" }
```

### 14.3 Architecture Design

```
┌────────────────────────────────────────────────────────────────────┐
│                        PhotonCast App                              │
├────────────────────────────────────────────────────────────────────┤
│  Extension Manager                                                 │
│  ├── Native Extension Loader (libloading) ──→ Direct Rust calls    │
│  └── Raycast Extension Bridge                                      │
│      │                                                             │
│      ├── Extension Detector                                        │
│      │   └── Identifies Raycast extensions by package.json         │
│      │                                                             │
│      ├── Build Pipeline                                            │
│      │   └── esbuild bundling (if not pre-bundled)                 │
│      │                                                             │
│      ├── Node.js Sidecar (one per extension for isolation)         │
│      │   ├── V8 Worker Thread (runs extension code)                │
│      │   ├── @photoncast/raycast-compat shim                       │
│      │   └── JSON-RPC handler                                      │
│      │                                                             │
│      └── Host Protocol Bridge                                      │
│          └── Translates JSON-RPC ←→ PhotonCast host services       │
└────────────────────────────────────────────────────────────────────┘
```

### 14.4 Sidecar Lifecycle

**Decision: One Node.js process per extension**

| Approach | Pros | Cons |
|----------|------|------|
| Shared process | Lower memory | Cross-extension interference, crash affects all |
| **Per-extension** | **Isolation, independent crashes** | Higher memory (~30-50MB per process) |

**Lifecycle states:**
```
Dormant → Starting → Ready → Active → Stopping → Stopped
```

**Process management:**
- Lazy spawn: Start Node process on first command invocation
- Idle timeout: Terminate after 5 minutes of inactivity
- Crash recovery: Restart up to 3 times, then mark Failed
- Memory limit: Kill if heap exceeds 512MB

### 14.5 Dependency Resolution & Bundling

**Raycast's approach (which we mirror):**

```
┌─────────────────────────────────────────────────────────────────┐
│                     Development Time                             │
├─────────────────────────────────────────────────────────────────┤
│  extension/                                                      │
│  ├── src/command.tsx      (TypeScript + React)                   │
│  ├── package.json         (declares dependencies)                │
│  └── node_modules/        (installed via npm install)            │
│      ├── axios/                                                  │
│      ├── lodash/                                                 │
│      └── @raycast/api/    (types only)                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │  Bundle (esbuild)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Runtime Bundle                               │
├─────────────────────────────────────────────────────────────────┤
│  dist/command.js          (single file, ~50-500KB)               │
│  ├── axios code           (inlined)                              │
│  ├── lodash code          (inlined)                              │
│  └── @raycast/api         (EXTERNAL - not bundled)               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │  Runtime
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     PhotonCast Host                              │
├─────────────────────────────────────────────────────────────────┤
│  Node.js Sidecar                                                 │
│  ├── @photoncast/raycast-compat (replaces @raycast/api)          │
│  ├── react (provided by host)                                    │
│  └── Loads dist/command.js in Worker thread                      │
└─────────────────────────────────────────────────────────────────┘
```

**External packages (provided by host, not bundled):**
- `@raycast/api` → replaced by `@photoncast/raycast-compat`
- `react` / `react-reconciler` → provided by host

**Benefits:**
- Run existing Raycast extensions unchanged
- Version consistency (host controls API version)
- Security (bundled code is inspectable)
- Performance (single-file bundles load fast)
- Offline operation (no runtime npm fetches)

### 14.6 API Compatibility Shim

The `@photoncast/raycast-compat` package implements the Raycast API surface:

```typescript
// @photoncast/raycast-compat/index.ts

// Re-export all Raycast API components and functions
export { List, Detail, Form, Grid, ActionPanel, Action } from "./components";
export { showToast, showHUD, closeMainWindow } from "./feedback";
export { Clipboard, LocalStorage, Cache } from "./storage";
export { environment, getPreferenceValues } from "./environment";
export { open, openExtensionPreferences } from "./actions";
export { launchCommand } from "./launch";
export * from "./types";

// Internal: IPC bridge to PhotonCast host
import { ipc } from "./ipc";

export function showToast(options: Toast.Options): Promise<Toast> {
  return ipc.call("showToast", options);
}

export const Clipboard = {
  copy: (text: string) => ipc.call("clipboard.copy", { text }),
  paste: (text: string) => ipc.call("clipboard.paste", { text }),
  read: () => ipc.call("clipboard.read", {}),
};
```

**Component rendering:**

React components are serialized to our declarative schema:

```typescript
// Extension code (unchanged from Raycast)
<List>
  <List.Item title="Hello" subtitle="World" />
</List>

// Serialized to JSON-RPC message:
{
  "method": "render",
  "params": {
    "view": {
      "type": "List",
      "items": [
        { "id": "0", "title": "Hello", "subtitle": "World" }
      ]
    }
  }
}

// PhotonCast host renders using native ListView
```

### 14.7 Manifest Translation

Raycast `package.json` → PhotonCast internal representation:

```json
// Raycast package.json
{
  "name": "github",
  "title": "GitHub",
  "description": "Search repositories",
  "icon": "github-icon.png",
  "author": "raycast",
  "license": "MIT",
  "commands": [
    {
      "name": "search-repos",
      "title": "Search Repositories",
      "subtitle": "GitHub",
      "description": "Search GitHub repositories",
      "mode": "view",
      "keywords": ["gh", "repo"]
    }
  ],
  "preferences": [
    {
      "name": "token",
      "title": "Personal Access Token",
      "type": "password",
      "required": true
    }
  ],
  "dependencies": {
    "@raycast/api": "^1.50.0",
    "octokit": "^2.0.0"
  }
}
```

**Translation rules:**

| Raycast Field | PhotonCast Equivalent |
|---------------|----------------------|
| `name` | `extension.id` (prefixed: `com.raycast.{name}`) |
| `title` | `extension.name` |
| `commands[].mode: "view"` | `commands[].mode = "view"` |
| `commands[].mode: "no-view"` | `commands[].mode = "no-view"` |
| `preferences[].type: "password"` | `preferences[].type = "secret"` |
| `preferences[].type: "textfield"` | `preferences[].type = "string"` |
| `preferences[].type: "dropdown"` | `preferences[].type = "select"` |

### 14.8 Host Services Parity

Required Raycast APIs to implement in the shim:

| Raycast API | Priority | PhotonCast Implementation |
|-------------|----------|---------------------------|
| `List`, `Detail`, `Form`, `Grid` | P0 | Map to ExtensionView schema |
| `Action`, `ActionPanel` | P0 | Map to Action system |
| `showToast` | P0 | `ExtensionHost::show_toast` |
| `Clipboard.copy/read` | P0 | `ExtensionHost::copy_to_clipboard` |
| `LocalStorage` | P0 | `ExtensionStorage` |
| `environment` | P0 | Extension context info |
| `getPreferenceValues` | P0 | `PreferenceStore` |
| `open` / `openExtensionPreferences` | P0 | `ExtensionHost::open_url` |
| `showHUD` | P1 | New: minimal HUD overlay |
| `Cache` | P1 | New: in-memory + disk cache |
| `launchCommand` | P1 | Cross-extension invocation |
| `getFrontmostApplication` | P2 | macOS accessibility API |
| `Keyboard.type` | P2 | Synthetic keyboard input |
| `AI` | P3 | Optional: integrate with local/cloud AI |

### 14.9 Sprint 6 Preparation

To prepare for Phase 3 during Sprint 6, ensure:

1. **Serializable view types**: Add `#[derive(Serialize, Deserialize)]` to all view/action structs
2. **Protocol abstraction**: Define `ExtensionHostProtocol` trait that both native and sidecar can implement
3. **Missing host services**: Add `showHUD`, `Cache` API, `getFrontmostApplication` to spec
4. **launchCommand**: Design cross-extension command invocation

```rust
// Ensure all view types are serializable for JSON-RPC transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionView {
    List(ListView),
    Detail(DetailView),
    Form(FormView),
    Grid(GridView),
}

// Protocol that both native extensions and Node sidecar implement
pub trait ExtensionHostProtocol: Send + Sync {
    fn render_view(&self, view: ExtensionView) -> Result<ViewHandle>;
    fn update_view(&self, handle: ViewHandle, patch: ViewPatch) -> Result<()>;
    fn show_toast(&self, toast: Toast) -> Result<()>;
    fn show_hud(&self, message: &str) -> Result<()>;
    fn copy_to_clipboard(&self, text: &str) -> Result<()>;
    fn read_clipboard(&self) -> Result<Option<String>>;
    fn open_url(&self, url: &str) -> Result<()>;
    fn get_preferences(&self) -> Result<PreferenceValues>;
    fn launch_command(&self, extension_id: &str, command_id: &str) -> Result<()>;
    // ... etc
}
```

### 14.10 Open Questions for Phase 3

| Question | Options | Leaning |
|----------|---------|---------|
| Node.js version management | Bundle with app vs. system Node vs. managed download | Managed download (like Raycast) |
| Extension store | Mirror Raycast store vs. separate vs. manual install only | Manual install + curated list |
| React version | Match Raycast's React version exactly | Yes, for maximum compatibility |
| Menu bar commands | Support Raycast menu bar extensions | Defer to Phase 4 |
| AI API | Implement Raycast AI compatibility | Defer, assess demand |

### 14.11 Implementation Phases

```
Phase 3a: Core Sidecar Infrastructure
├── Node.js process management
├── JSON-RPC transport layer  
├── Basic component rendering (List, Detail)
└── Essential host services (toast, clipboard, storage)

Phase 3b: Full API Compatibility  
├── All view components (Form, Grid, ActionPanel)
├── Navigation API (push/pop)
├── Preferences system
└── Cache API

Phase 3c: Developer Experience
├── Extension detection and auto-loading
├── Hot-reload for Raycast extensions
├── Error reporting with source maps
└── Debug tooling

Phase 3d: Polish & Compatibility Testing
├── Test against top 50 Raycast extensions
├── Performance optimization
├── Documentation
└── Migration guide for extension authors
```
