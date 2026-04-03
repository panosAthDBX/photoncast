# PhotonCast Phase 2: Version 1.0 - Productivity Features

> **Specification Document**  
> Version: 1.0.0  
> Date: 2026-01-16  
> Timeline: Months 4-6 (Sprints 4-6, Weeks 13-24)  
> Release Target: v1.0.0

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Goals and Non-Goals](#2-goals-and-non-goals)
3. [Feature Specifications](#3-feature-specifications)
4. [Technical Architecture](#4-technical-architecture)
5. [Data Models and Storage](#5-data-models-and-storage)
6. [UI/UX Specifications](#6-uiux-specifications)
7. [Extension API Contracts](#7-extension-api-contracts)
8. [Security Considerations](#8-security-considerations)
9. [Performance Requirements](#9-performance-requirements)
10. [Testing Strategy](#10-testing-strategy)
11. [Dependencies and Crates](#11-dependencies-and-crates)
12. [Risks and Mitigations](#12-risks-and-mitigations)
13. [Success Metrics](#13-success-metrics)

---

## 1. Executive Summary

### 1.1 Overview

Phase 2 transforms PhotonCast from a functional MVP (Phase 1) into a feature-complete v1.0 release with parity to basic Raycast/Alfred use cases. This phase spans 12 weeks (3 sprints) and delivers the core productivity features that power users expect from a modern macOS launcher.

### 1.2 Key Deliverables

| Sprint | Focus | Key Features |
|--------|-------|--------------|
| Sprint 4 (Weeks 13-16) | Productivity Features | Clipboard History, Calculator |
| Sprint 5 (Weeks 17-20) | Window & System | Window Management, Calendar, App Management, Quick Links, Sleep Timer, Preferences |
| Sprint 6 (Weeks 21-24) | Extensions | Native Extension System, Custom Commands, First-Party Extensions |

### 1.3 Critical Decisions

Based on requirements analysis, the following key decisions have been made:

- **ALL features are MUST-HAVE** for Phase 2 exit (no nice-to-haves)
- **Encrypted SQLite** for clipboard history with AES-256 encryption
- **evalexpr + frankfurter.app + CoinGecko** for calculator functionality
- **Read-only calendar** for v1.0 (event creation deferred to Phase 3+)
- **Deep scan ON by default** for app uninstall
- **App Sleep feature** - stop apps after configurable idle time
- **Arc browser import** added to Quick Links alongside Safari/Chrome/Firefox
- **Enter joins meeting** as primary action for calendar events with conference links
- **Permissioned sandbox** for extensions with async-first API
- **4 Catppuccin themes** with 14 accent color options
- **80% test coverage** target with integration tests for all features

### 1.4 Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (Edition 2021, MSRV 1.75+) |
| GUI Framework | GPUI + gpui-component |
| Async Runtime | Tokio |
| Storage | SQLite (rusqlite) + TOML configs |
| Config Location | `~/.config/photoncast/config.toml` |
| Extension Location | `~/Library/Application Support/PhotonCast/Extensions/` |

---

## 2. Goals and Non-Goals

### 2.1 Goals

1. **Feature Parity** - Match basic Raycast/Alfred use cases for daily productivity
2. **Performance** - Maintain <50ms response times for all operations
3. **Privacy-First** - Local-only data, encrypted storage, no telemetry
4. **Extensibility** - Robust native extension system with Raycast API compatibility
5. **Quality** - 80% test coverage, comprehensive integration tests
6. **Polish** - 4 themes, 14 accents, smooth 200ms animations

### 2.2 Non-Goals (Phase 2)

1. **Cloud Sync** - No iCloud/cross-device sync for clipboard or settings
2. **Event Creation** - Calendar is read-only in v1.0
3. **Clipboard Merge** - Combining multiple clipboard items deferred to Phase 3+
4. **OCR** - Text extraction from images deferred to Phase 3+
5. **Calculator Variables** - Session variable assignment deferred to Phase 3+
6. **Interactive Commands** - stdin support for custom commands deferred
7. **Menu Bar Mode** - Extensions with menu-bar mode have limited support
8. **AI Features** - No AI/LLM integration (by design)

### 2.3 Deferred to Phase 3+

| Feature | Reason |
|---------|--------|
| Clipboard item merging | Focus on core features first |
| OCR for clipboard images | Complex, requires macOS Vision framework |
| Calculator variable assignment | Keep calculator simple initially |
| Interactive custom commands | Complex UX |
| Calendar event creation | Requires read-write access, more complex |
| Focus mode integration | Complex feature |
| Display disconnect/restore | Let macOS handle |
| Custom grid layouts | Preset layouts sufficient for v1.0 |

---

## 3. Feature Specifications

### 3.1 Sprint 4: Productivity Features (Weeks 13-16)

#### 3.1.1 Clipboard History

**User Story:** As a power user, I want to access my clipboard history so that I can reuse previously copied content without manually re-copying.

**Core Requirements:**

| Requirement | Specification |
|-------------|---------------|
| Global Hotkey | `Cmd+Shift+V` (default) |
| Retention Period | 30 days |
| Max Items | 1000 (configurable) |
| Storage | Encrypted SQLite with AES-256 |
| Encryption Key | Machine-derived key |

**Content Types:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardContentType {
    /// Plain text content
    Text {
        content: String,
        preview: String, // First 100 chars
    },
    /// Rich text with HTML/RTF
    RichText {
        plain: String,
        html: Option<String>,
        rtf: Option<String>,
    },
    /// Image content
    Image {
        path: PathBuf,        // Stored image path
        thumbnail_path: PathBuf,
        size_bytes: u64,
        dimensions: (u32, u32),
    },
    /// File references
    File {
        paths: Vec<PathBuf>,
        icons: Vec<PathBuf>,
        total_size: u64,
    },
    /// URL with metadata
    Link {
        url: String,
        title: Option<String>,
        favicon_path: Option<PathBuf>,
    },
    /// Color value
    Color {
        hex: String,
        rgb: (u8, u8, u8),
        display_name: Option<String>,
    },
}
```

**Privacy & Security:**

- **Excluded Apps (Default):**
  - `com.1password.1password` (1Password)
  - `com.agilebits.onepassword7` (1Password 7)
  - `com.bitwarden.desktop` (Bitwarden)
  - `com.lastpass.LastPass` (LastPass)
  - `com.apple.keychainaccess` (Keychain Access)
  - `com.dashlane.Dashlane` (Dashlane)
- **Transient Items:** Never stored (respects NSPasteboard transient flag)
- **Concealed Mode:** "Paste and Don't Save" action available
- **Clear History:** Requires confirmation dialog

**Features:**

| Feature | Specification |
|---------|---------------|
| Pinned Items | Separate "Pinned" section, don't count against limit |
| Enter Action | Configurable: "Paste directly" (default) or "Copy to clipboard" |
| Image Max Size | 10MB (configurable) |
| URL Preview | Fetch favicon + title in background with caching |
| Color Detection | Display color swatch preview for hex/rgb values |
| Search | Full-text search across all text content |

**Actions:**

| Action | Shortcut | Description |
|--------|----------|-------------|
| Paste | `Enter` | Paste item directly (default) |
| Copy | `Cmd+C` | Copy to clipboard without pasting |
| Paste as Plain Text | `Cmd+Shift+V` | Strip formatting |
| Paste and Don't Save | `Cmd+Opt+V` | One-time paste, not saved |
| Pin/Unpin | `Cmd+P` | Toggle pinned status |
| Delete | `Cmd+Backspace` | Remove from history |
| Clear All | `Cmd+Shift+Backspace` | Clear entire history (with confirmation) |

---

#### 3.1.2 Built-in Calculator

**User Story:** As a user, I want to perform calculations, conversions, and date/time operations directly from the launcher.

**Expression Parsing:**

```rust
// Using evalexpr crate for math evaluation
use evalexpr::{eval, Context, HashMapContext};

pub struct Calculator {
    /// Currency exchange rates (updated every 6 hours)
    currency_rates: HashMap<String, f64>,
    /// Cryptocurrency rates (from CoinGecko)
    crypto_rates: HashMap<String, f64>,
    /// Last rate update timestamp
    last_update: DateTime<Utc>,
    /// City timezone database (~500 cities)
    city_timezones: HashMap<String, Tz>,
}
```

**Math Operations:**

| Category | Functions |
|----------|-----------|
| Arithmetic | `+`, `-`, `*`, `/`, `^`, `%` |
| Basic Functions | `sqrt`, `abs`, `floor`, `ceil`, `round` |
| Trigonometric | `sin`, `cos`, `tan`, `asin`, `acos`, `atan` |
| Hyperbolic | `sinh`, `cosh`, `tanh` |
| Logarithmic | `log`, `ln`, `exp` |
| Other | `pow`, `mod`, `min`, `max`, `factorial` |
| Constants | `pi`, `e` |

**Precision:**
- General math: `f64` (standard)
- Currency calculations: `Decimal128` (high precision)

**Currency Conversion:**

| API | Purpose | Update Frequency |
|-----|---------|------------------|
| frankfurter.app | Fiat currencies (150+) | Every 6 hours |
| CoinGecko | Cryptocurrencies | Every 6 hours |

**Supported Cryptocurrencies:**
BTC, ETH, USDT, BNB, XRP, ADA, DOGE, SOL, USDC, MATIC, AVAX, DOT, LINK

**Unit Conversions:**

| Category | Units |
|----------|-------|
| Length | mm, cm, m, km, in, ft, yd, mi |
| Weight | mg, g, kg, oz, lb, ton |
| Volume | ml, l, tsp, tbsp, cup, pt, qt, gal |
| Temperature | C, F, K |
| Data | B, KB, MB, GB, TB, PB |
| Speed | m/s, km/h, mph, knots, ft/s |

- **Case-insensitive:** `5 km` = `5 KM` = `5 Km`
- **Aliases supported:** "kilometers", "km", "kms", "kilometre"

**Date/Time:**

```rust
// Natural language date parsing
pub fn parse_date_expression(input: &str) -> Result<DateTimeResult> {
    // Supported formats:
    // - "monday in 3 weeks"
    // - "35 days ago"
    // - "days until dec 25"
    // - "time in dubai"
    // - "5pm ldn in sf"
    // - "2pm est to pst"
}
```

- **Timezone Resolution:** Common US/EU conventions with system locale fallback
- **City Database:** Bundled ~500 cities mapped to IANA timezone identifiers

**Calculator UX:**

| Behavior | Specification |
|----------|---------------|
| Real-time evaluation | Yes, with debounce for expensive calculations |
| Copy result | `Enter` = formatted, `Cmd+Enter` = raw number |
| History | Separate "Calculator History" command |
| Offline support | SQLite cache with "rates as of X" display |

---

### 3.2 Sprint 5: Window Management & Productivity (Weeks 17-20)

#### 3.2.1 Window Management

**User Story:** As a user, I want to position and resize windows with keyboard commands for efficient screen organization.

**Layouts:**

```rust
#[derive(Debug, Clone, Copy)]
pub enum WindowLayout {
    // Halves
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    
    // Quarters
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    
    // Thirds
    FirstThird,
    CenterThird,
    LastThird,
    FirstTwoThirds,
    LastTwoThirds,
    
    // Full/Restore
    Maximize,
    Center,
    Restore,
}
```

**Cycling Behavior:**
- Press "Left Half" twice → cycles between 50% → 33% → 66% width
- Matches Magnet/Rectangle behavior

**Multi-Monitor:**

| Command | Behavior |
|---------|----------|
| Move to Next Display | Cycle by macOS arrangement order |
| Move to Previous Display | Cycle reverse |
| Move to Display N | Move to specific monitor (1, 2, 3) |
| Position Preservation | Preserve relative position (Left Half → Left Half) |

**Animation:**

| Setting | Value |
|---------|-------|
| Duration | 200ms |
| Reduce Motion | Respect macOS accessibility setting |
| Configurable | On/off toggle in preferences |

**Permissions:**
- Requires Accessibility permission
- Prompt on first use + inline status in command results

**No Custom Grid Layouts** for v1.0 (preset layouts only)
**No Snap to Edges** (command-only window management)

---

#### 3.2.2 Quick Links

**User Story:** As a user, I want to create custom URL shortcuts with keywords for instant access to frequently visited sites.

**Storage:**
- Primary: SQLite database (for UI editing)
- Export: TOML file (for backup/sharing/version control)

```toml
# ~/.config/photoncast/quicklinks.toml

[[links]]
title = "GitHub"
url = "https://github.com"
keywords = ["gh", "git"]
icon = "github"
tags = ["dev"]

[[links]]
title = "GitHub Search"
url = "https://github.com/search?q={query}"  # Dynamic parameter
keywords = ["ghs"]
tags = ["dev"]
```

**Features:**

| Feature | Specification |
|---------|---------------|
| Dynamic URLs | Support `{query}` placeholder with input prompt |
| Organization | Tag-based (more flexible than folders) |
| Favicon Display | Auto-fetch and cache |
| Browser Import | Safari, Chrome, Firefox, **Arc** |

**Browser Import Sources:**

| Browser | Source Location |
|---------|-----------------|
| Safari | `~/Library/Safari/Bookmarks.plist` |
| Chrome | `~/Library/Application Support/Google/Chrome/Default/Bookmarks` |
| Firefox | `~/Library/Application Support/Firefox/Profiles/*/places.sqlite` |
| Arc | `~/Library/Application Support/Arc/StorableSidebar.json` |

---

#### 3.2.3 Calendar Integration

**User Story:** As a user, I want to view my calendar events and join meetings quickly from the launcher.

**Access Level:** Read-only for v1.0

**Commands:**

| Command | Description |
|---------|-------------|
| My Schedule | View 7 days of upcoming events |
| Today's Events | Events for current day |
| This Week | Events for current week |

**Event Display:**

```rust
#[derive(Debug)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub attendees: Vec<Attendee>,
    pub conference_url: Option<String>,
    pub calendar_color: Color,
    pub calendar_name: String,
}
```

- All-day events grouped at top of each day
- Recurring events shown as individual instances
- Timezone conversion to local time with indicator if different

**Conference Detection:**

| Provider | URL Pattern |
|----------|-------------|
| Zoom | `zoom.us/j/`, `zoom.us/my/` |
| Google Meet | `meet.google.com/` |
| Microsoft Teams | `teams.microsoft.com/l/meetup-join/` |

Detection locations: Location field, Notes, Structured conference data

**Join Meeting Behavior:**
- **Enter key joins meeting** for events with conference links (primary action)
- Join action shown whenever a conference link is detected for the selected event
- Opens conference URL in default browser

**Permission Handling:**
- Request on first calendar command (least intrusive)
- Uses EventKit framework

**Not in v1.0:** Event creation, Focus mode integration

---

#### 3.2.4 App Management

**User Story:** As a user, I want to manage applications including uninstalling with cleanup and controlling running apps.

**Uninstaller:**

```rust
pub struct UninstallPreview {
    pub app: Application,
    pub related_files: Vec<RelatedFile>,
    pub total_size: u64,
    pub space_freed_formatted: String,
}

pub struct RelatedFile {
    pub path: PathBuf,
    pub size: u64,
    pub category: RelatedFileCategory,
}

pub enum RelatedFileCategory {
    ApplicationSupport,  // ~/Library/Application Support/<App>
    Preferences,         // ~/Library/Preferences/<bundle-id>.plist
    Caches,              // ~/Library/Caches/<bundle-id>
    Logs,                // ~/Library/Logs/<App>
    SavedState,          // ~/Library/Saved Application State/<bundle-id>.savedState
    Containers,          // ~/Library/Containers/<bundle-id>
}
```

**Uninstall Behavior:**

| Setting | Value |
|---------|-------|
| Confirmation | Yes, with preview of files to delete |
| System App Protection | Hardcoded for `/System/Applications` |
| Space Display | Show space to be freed before uninstall |
| Cleanup Approach | Conservative (exact bundle ID matches only) |
| Deep Scan | **ON by default** (user can disable) |
| Destination | Move to Trash (safer, allows recovery) |

**Force Quit:**

| Feature | Specification |
|---------|---------------|
| Confirmation | Required for non-frozen apps, skipped if unresponsive |
| Not Responding | Show indicator for hung apps |
| Quit vs Force Quit | Both prominently available as actions |

**App Sleep Feature (NEW):**

```rust
pub struct AppSleepConfig {
    /// Enable app sleep feature
    pub enabled: bool,
    /// Default idle timeout before sleeping
    pub default_idle_minutes: u32,
    /// Per-app overrides
    pub app_overrides: HashMap<BundleId, AppSleepOverride>,
}

pub struct AppSleepOverride {
    /// Custom idle timeout for this app
    pub idle_minutes: Option<u32>,
    /// Never sleep this app
    pub never_sleep: bool,
}
```

Stop apps after X minutes of idle time (configurable per-app).

---

#### 3.2.5 Sleep Timer

**User Story:** As a user, I want to schedule system actions like sleep or shutdown after a delay.

**Supported Actions:**

| Action | Example Command |
|--------|-----------------|
| Sleep | "Sleep in 30 minutes" |
| Shut Down | "Shutdown at 10pm" |
| Restart | "Restart in 1 hour" |
| Lock | "Lock in 15 minutes" |

**Behavior:**

| Setting | Value |
|---------|-------|
| Persistence | Survives PhotonCast restarts |
| Warning | 1 minute before with cancel option |
| Multiple Timers | No (one timer at a time) |
| Countdown Display | Menu bar + launcher when open |
| Activity Cancel | No (strict timer, user explicitly set it) |

**Natural Language Parsing:**
- Minutes: `5 min`, `15 minutes`, `30m`
- Hours: `1 hour`, `2h`, `1.5 hours`
- Time: `at 10pm`, `at 22:00`

---

#### 3.2.6 Preferences & Settings

**User Story:** As a user, I want to customize PhotonCast's behavior, appearance, and keyboard shortcuts.

**Storage:**

```
~/.config/photoncast/
├── config.toml          # Main configuration
├── quicklinks.toml      # Quick links (exportable)
└── keybindings.toml     # Custom keyboard shortcuts
```

**Theme System:**

| Theme | Description |
|-------|-------------|
| Catppuccin Latte | Light theme |
| Catppuccin Frappé | Dark - low contrast |
| Catppuccin Macchiato | Dark - medium contrast |
| Catppuccin Mocha | Dark - high contrast (default) |
| Auto | Follow system appearance |

**Accent Colors (14):**

```rust
pub enum AccentColor {
    Rosewater,
    Flamingo,
    Pink,
    Mauve,
    Red,
    Maroon,
    Peach,
    Yellow,
    Green,
    Teal,
    Sky,
    Sapphire,
    Blue,      // Default
    Lavender,
}
```

**Configuration Schema:**

```toml
# ~/.config/photoncast/config.toml

[general]
launch_at_login = true
global_hotkey = "Cmd+Space"

[appearance]
theme = "mocha"           # latte, frappe, macchiato, mocha, auto
accent_color = "blue"
window_animation = true
animation_duration_ms = 200

[clipboard]
enabled = true
hotkey = "Cmd+Shift+V"
history_size = 1000
retention_days = 30
store_images = true
max_image_size_mb = 10
excluded_apps = [
    "com.1password.1password",
    "com.bitwarden.desktop",
]
default_action = "paste"  # paste, copy

[calculator]
currency_update_hours = 6
show_history = true

[window_management]
animation_enabled = true
animation_duration_ms = 200
cycling_enabled = true

[calendar]
days_ahead = 7
show_all_day_first = true

[app_management]
deep_scan_default = true

[app_sleep]
enabled = false
default_idle_minutes = 30

[sleep_timer]
warning_minutes = 1
show_in_menu_bar = true
```

**Keyboard Shortcuts:**
- Fully customizable
- Hyper key support (Cmd+Ctrl+Opt+Shift)
- No default hotkeys for window layouts (avoid conflicts)
- Suggested mappings provided in preferences UI

---

### 3.3 Sprint 6: Native Extension System (Weeks 21-24)

#### 3.3.1 Extension Architecture

**User Story:** As a developer, I want to create native Rust extensions that integrate deeply with PhotonCast.

**Extension Location:**
```
~/Library/Application Support/PhotonCast/Extensions/
├── my-extension/
│   ├── extension.toml    # Manifest
│   ├── src/
│   │   └── lib.rs
│   └── Cargo.toml
```

**Manifest Format:**

```toml
# extension.toml

[extension]
name = "my-extension"
title = "My Extension"
description = "Does something useful"
version = "1.0.0"
author = "username"
license = "MIT"
icon = "icon.png"

# Required permissions
[permissions]
clipboard_read = true
clipboard_write = true
network = true
filesystem_read = true    # Scoped to user directory
filesystem_write = false  # Scoped to extension directory
notifications = true
storage = true

[[commands]]
name = "main"
title = "Main Command"
description = "The main command"
mode = "view"            # view, no-view
icon = "command-icon.png"

[[commands]]
name = "quick-action"
title = "Quick Action"
mode = "no-view"

[preferences]
[[preferences.items]]
name = "apiKey"
type = "password"
required = true
title = "API Key"
description = "Your API key"

[[preferences.items]]
name = "showDetails"
type = "checkbox"
required = false
title = "Show Details"
default = true
```

**Permission Model:**

| Permission | Scope |
|------------|-------|
| `clipboard_read` | Read clipboard content |
| `clipboard_write` | Write to clipboard |
| `network` | Make HTTP requests |
| `filesystem_read` | Read files (scoped to user directory) |
| `filesystem_write` | Write files (scoped to extension directory only) |
| `notifications` | Show system notifications |
| `storage` | Per-extension persistent storage (SQLite) |

**Sandboxing:**
- Permissioned sandbox model
- Extensions declare required permissions
- Isolated process per extension (with error boundaries as fallback)

**API Design:**
- **Async-first** (modern Rust patterns)
- Extensions can provide search results
- Error handling: Isolated processes + error boundaries

---

#### 3.3.2 Extension API

```rust
//! PhotonCast Extension API

use photoncast_extension_api::prelude::*;

/// Core extension trait
#[async_trait]
pub trait Extension: Send + Sync {
    /// Extension metadata
    fn manifest(&self) -> &ExtensionManifest;
    
    /// Called when extension is loaded
    async fn on_load(&mut self, ctx: &ExtensionContext) -> Result<()>;
    
    /// Called when extension is unloaded
    async fn on_unload(&mut self) -> Result<()>;
}

/// Context provided to extensions
pub struct ExtensionContext {
    /// Extension storage (SQLite)
    pub storage: ExtensionStorage,
    /// Clipboard access (if permitted)
    pub clipboard: Option<ClipboardAccess>,
    /// HTTP client (if permitted)
    pub http: Option<HttpClient>,
    /// Notification API (if permitted)
    pub notifications: Option<NotificationApi>,
    /// Preferences values
    pub preferences: HashMap<String, Value>,
}

/// UI Components available to extensions
pub enum ExtensionView {
    List(ListView),
    Grid(GridView),
    Detail(DetailView),
    Form(FormView),
}

/// List component
pub struct ListView {
    pub is_loading: bool,
    pub search_placeholder: Option<String>,
    pub items: Vec<ListItem>,
    pub sections: Vec<ListSection>,
    pub empty_view: Option<EmptyView>,
}

pub struct ListItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<Icon>,
    pub accessories: Vec<Accessory>,
    pub actions: Vec<Action>,
    pub keywords: Vec<String>,
}

/// Actions
pub struct Action {
    pub title: String,
    pub icon: Option<Icon>,
    pub shortcut: Option<Shortcut>,
    pub style: ActionStyle,
    pub handler: ActionHandler,
}

pub enum ActionHandler {
    OpenUrl(String),
    CopyToClipboard(String),
    Paste(String),
    Push(Box<dyn Fn() -> ExtensionView>),
    Pop,
    Custom(Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>>>>>),
}

/// Toast notifications
pub async fn show_toast(options: ToastOptions) -> Result<Toast>;
pub async fn show_hud(title: &str) -> Result<()>;

pub struct ToastOptions {
    pub style: ToastStyle,
    pub title: String,
    pub message: Option<String>,
    pub primary_action: Option<ToastAction>,
}

pub enum ToastStyle {
    Success,
    Failure,
    Animated,
}
```

**Storage API:**

```rust
/// Per-extension persistent storage
pub struct ExtensionStorage {
    extension_id: String,
    db: SqliteConnection,
}

impl ExtensionStorage {
    pub async fn get(&self, key: &str) -> Result<Option<String>>;
    pub async fn set(&self, key: &str, value: &str) -> Result<()>;
    pub async fn remove(&self, key: &str) -> Result<()>;
    pub async fn all(&self) -> Result<HashMap<String, String>>;
    pub async fn clear(&self) -> Result<()>;
}
```

---

#### 3.3.3 Hot Reload & Development

**Development Mode:**

```bash
# Extension development CLI
photoncast extension new my-extension      # Create new extension
photoncast extension dev                   # Run with hot-reload
photoncast extension build                 # Build for distribution
photoncast extension validate              # Validate manifest and code
photoncast extension package               # Package for distribution
```

**Hot Reload:**
- File watcher for `.rs` files in dev mode
- Automatic recompilation and reload
- Preserves extension state where possible

---

#### 3.3.4 Custom Commands

**User Story:** As a power user, I want to create custom command shortcuts that execute shell scripts.

**Configuration:**

```toml
# ~/.config/photoncast/commands.toml

[[commands]]
name = "deploy-staging"
title = "Deploy to Staging"
icon = "rocket"
shell = "/bin/zsh"           # Defaults to $SHELL
script = """
cd ~/projects/myapp
./deploy.sh staging
"""
timeout_seconds = 60
environment = { DEPLOY_ENV = "staging" }

[[commands]]
name = "docker-cleanup"
title = "Docker Cleanup"
icon = "trash"
script = "docker system prune -af"
confirm = true
```

**Behavior:**

| Setting | Value |
|---------|-------|
| Shell | Configurable per command, defaults to `$SHELL` |
| Output | Streamed (real-time) |
| Timeout | Configurable, 60 seconds default |
| Environment | Inherit system + per-command additions |
| Notifications | HUD for success, Toast for failure |
| Interactive (stdin) | Not supported in v1.0 |

---

#### 3.3.5 First-Party Extensions

**Bundled Extensions:**

| Extension | Description |
|-----------|-------------|
| GitHub Repositories | Browse and search your repos |
| System Preferences | Quick access to macOS settings |
| Screenshots Browser | Browse and search recent screenshots |

**Bundled vs Downloaded:** All first-party extensions bundled (better first-run experience)

---

## 4. Technical Architecture

### 4.1 High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                        PhotonCast Application                         │
├──────────────────────────────────────────────────────────────────────┤
│                           GPUI UI Layer                               │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐ │
│  │ Search Bar  │ │Results List │ │ Detail View │ │  Action Panel   │ │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────────┘ │
├──────────────────────────────────────────────────────────────────────┤
│                         Feature Modules                               │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────────┐ │
│ │ Clipboard  │ │ Calculator │ │  Calendar  │ │ Window Management  │ │
│ └────────────┘ └────────────┘ └────────────┘ └────────────────────┘ │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────────┐ │
│ │Quick Links │ │App Manager │ │Sleep Timer │ │    Preferences     │ │
│ └────────────┘ └────────────┘ └────────────┘ └────────────────────┘ │
├──────────────────────────────────────────────────────────────────────┤
│                       Extension Host                                  │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │   Extension Manager   │   Sandbox   │   IPC Bridge             │ │
│  └─────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────┤
│                      Platform Integration                             │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────────┐ │
│ │  Hotkey    │ │Accessibility│ │  EventKit  │ │    NSPasteboard   │ │
│ └────────────┘ └────────────┘ └────────────┘ └────────────────────┘ │
├──────────────────────────────────────────────────────────────────────┤
│                         Storage Layer                                 │
│  ┌───────────────────────────┐  ┌───────────────────────────────┐   │
│  │   SQLite (Encrypted)      │  │   TOML Config Files           │   │
│  │   - Clipboard History     │  │   - config.toml               │   │
│  │   - Quick Links           │  │   - quicklinks.toml           │   │
│  │   - Extension Storage     │  │   - commands.toml             │   │
│  │   - Currency Cache        │  │   - keybindings.toml          │   │
│  └───────────────────────────┘  └───────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

### 4.2 Module Structure

```
crates/
├── photoncast/                  # Main application
│   └── src/
│       ├── main.rs
│       ├── app/
│       │   ├── mod.rs
│       │   ├── state.rs
│       │   └── config.rs
│       └── ui/
│           ├── launcher.rs
│           └── components/
├── photoncast-clipboard/        # Clipboard history
│   └── src/
│       ├── lib.rs
│       ├── monitor.rs
│       ├── storage.rs
│       └── encryption.rs
├── photoncast-calculator/       # Calculator
│   └── src/
│       ├── lib.rs
│       ├── parser.rs
│       ├── evaluator.rs
│       ├── currency.rs
│       ├── units.rs
│       └── datetime.rs
├── photoncast-window/           # Window management
│   └── src/
│       ├── lib.rs
│       ├── layouts.rs
│       └── accessibility.rs
├── photoncast-calendar/         # Calendar integration
│   └── src/
│       ├── lib.rs
│       ├── eventkit.rs
│       └── conference.rs
├── photoncast-apps/             # App management
│   └── src/
│       ├── lib.rs
│       ├── uninstaller.rs
│       ├── force_quit.rs
│       └── app_sleep.rs
├── photoncast-quicklinks/       # Quick links
│   └── src/
│       ├── lib.rs
│       ├── storage.rs
│       └── import.rs
├── photoncast-timer/            # Sleep timer
│   └── src/
│       ├── lib.rs
│       └── scheduler.rs
├── photoncast-extensions/       # Extension system
│   └── src/
│       ├── lib.rs
│       ├── host.rs
│       ├── sandbox.rs
│       ├── api.rs
│       └── ipc.rs
└── photoncast-extension-api/    # Extension API (published crate)
    └── src/
        ├── lib.rs
        ├── components.rs
        ├── storage.rs
        └── prelude.rs
```

### 4.3 Async Architecture

All I/O operations use Tokio async runtime:

```rust
use tokio::sync::{mpsc, oneshot, RwLock};

pub struct ClipboardMonitor {
    /// Channel for clipboard events
    event_tx: mpsc::Sender<ClipboardEvent>,
    /// Storage handle
    storage: Arc<RwLock<ClipboardStorage>>,
    /// Monitoring state
    running: Arc<AtomicBool>,
}

impl ClipboardMonitor {
    pub async fn start(&self) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        
        while self.running.load(Ordering::SeqCst) {
            if let Some(item) = self.check_pasteboard().await? {
                self.storage.write().await.store(item).await?;
                self.event_tx.send(ClipboardEvent::NewItem).await?;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        
        Ok(())
    }
}
```

---

## 5. Data Models and Storage

### 5.1 Database Schema

**Clipboard History (Encrypted SQLite):**

```sql
-- Clipboard items
CREATE TABLE clipboard_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_type TEXT NOT NULL,  -- 'text', 'rich_text', 'image', 'file', 'link', 'color'
    
    -- Text content (encrypted)
    text_content BLOB,
    html_content BLOB,
    rtf_content BLOB,
    
    -- Image/File references
    image_path TEXT,
    thumbnail_path TEXT,
    file_paths TEXT,  -- JSON array
    
    -- Link metadata
    url TEXT,
    link_title TEXT,
    favicon_path TEXT,
    
    -- Color data
    color_hex TEXT,
    color_rgb TEXT,  -- "r,g,b"
    
    -- Metadata
    source_app TEXT,
    source_bundle_id TEXT,
    size_bytes INTEGER,
    is_pinned INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    accessed_at TEXT,
    
    -- Full-text search
    search_text TEXT
);

CREATE INDEX idx_clipboard_created ON clipboard_items(created_at DESC);
CREATE INDEX idx_clipboard_pinned ON clipboard_items(is_pinned DESC, created_at DESC);
CREATE VIRTUAL TABLE clipboard_fts USING fts5(search_text, content=clipboard_items, content_rowid=id);
```

**Quick Links (SQLite + TOML export):**

```sql
CREATE TABLE quick_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    keywords TEXT,  -- JSON array
    tags TEXT,      -- JSON array
    icon_path TEXT,
    favicon_path TEXT,
    created_at TEXT NOT NULL,
    accessed_at TEXT,
    access_count INTEGER DEFAULT 0
);

CREATE INDEX idx_quicklinks_title ON quick_links(title);
CREATE VIRTUAL TABLE quicklinks_fts USING fts5(title, url, keywords);
```

**Extension Storage (SQLite):**

```sql
CREATE TABLE extension_storage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    extension_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    
    UNIQUE(extension_id, key)
);

CREATE INDEX idx_extension_storage ON extension_storage(extension_id, key);
```

**Currency Cache (SQLite):**

```sql
CREATE TABLE currency_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    base_currency TEXT NOT NULL,
    target_currency TEXT NOT NULL,
    rate REAL NOT NULL,
    source TEXT NOT NULL,  -- 'frankfurter', 'coingecko'
    updated_at TEXT NOT NULL,
    
    UNIQUE(base_currency, target_currency)
);

CREATE INDEX idx_currency_rates ON currency_rates(base_currency, target_currency);
```

**Sleep Timer (SQLite for persistence):**

```sql
CREATE TABLE active_timer (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Single row
    action TEXT NOT NULL,
    execute_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

### 5.2 Configuration Files

**Main Config (`~/.config/photoncast/config.toml`):**

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub clipboard: ClipboardConfig,
    pub calculator: CalculatorConfig,
    pub window_management: WindowConfig,
    pub calendar: CalendarConfig,
    pub app_management: AppManagementConfig,
    pub app_sleep: AppSleepConfig,
    pub sleep_timer: SleepTimerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub launch_at_login: bool,
    pub global_hotkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub theme: Theme,
    pub accent_color: AccentColor,
    pub window_animation: bool,
    pub animation_duration_ms: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Latte,
    Frappe,
    Macchiato,
    Mocha,
    Auto,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                launch_at_login: true,
                global_hotkey: "Cmd+Space".to_string(),
            },
            appearance: AppearanceConfig {
                theme: Theme::Mocha,
                accent_color: AccentColor::Blue,
                window_animation: true,
                animation_duration_ms: 200,
            },
            clipboard: ClipboardConfig {
                enabled: true,
                hotkey: "Cmd+Shift+V".to_string(),
                history_size: 1000,
                retention_days: 30,
                store_images: true,
                max_image_size_mb: 10,
                excluded_apps: vec![
                    "com.1password.1password".to_string(),
                    "com.bitwarden.desktop".to_string(),
                ],
                default_action: ClipboardAction::Paste,
            },
            // ... other defaults
        }
    }
}
```

### 5.3 Encryption

**Clipboard Encryption:**

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

pub struct EncryptionManager {
    key: Key<Aes256Gcm>,
}

impl EncryptionManager {
    /// Derive key from machine-specific data
    pub fn new() -> Result<Self> {
        let machine_id = get_machine_id()?;
        let salt = b"photoncast-clipboard-v1";
        
        let mut key_bytes = [0u8; 32];
        argon2::Argon2::default()
            .hash_password_into(machine_id.as_bytes(), salt, &mut key_bytes)?;
        
        let key = Key::from_slice(&key_bytes).clone();
        Ok(Self { key })
    }
    
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = generate_nonce();
        let ciphertext = cipher.encrypt(&nonce, plaintext)?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }
    
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key);
        let (nonce, data) = ciphertext.split_at(12);
        let nonce = Nonce::from_slice(nonce);
        let plaintext = cipher.decrypt(nonce, data)?;
        Ok(plaintext)
    }
}
```

---

## 6. UI/UX Specifications

### 6.1 Design Language

**Visual Approach:** Similar to Raycast but distinct - familiar patterns with Catppuccin aesthetic

**Must-Match Raycast Behaviors:**
- Clipboard history keyboard shortcuts
- Calculator natural language parsing
- Window management layouts
- Calendar event display
- Enter joins meeting

### 6.2 Clipboard History UI

```
┌─────────────────────────────────────────────────────────────┐
│ 🔍 Search clipboard history...                              │
├─────────────────────────────────────────────────────────────┤
│ 📌 PINNED                                                   │
│ ├─ 📝 API Key: sk-abc...xyz                    2 hours ago │
│ └─ 🔗 https://github.com/...                   Yesterday   │
├─────────────────────────────────────────────────────────────┤
│ 📋 RECENT                                                   │
│ ├─ 📝 Hello, this is a longer text...          Just now    │
│ ├─ 🖼️ Screenshot.png (1.2 MB)                  5 min ago   │
│ ├─ 📁 Document.pdf, Spreadsheet.xlsx           10 min ago  │
│ ├─ 🎨 #FF5733                                  15 min ago  │
│ │     ██████ Orange/Red                                    │
│ └─ 🔗 https://example.com                      20 min ago  │
│       Example Domain - favicon                              │
└─────────────────────────────────────────────────────────────┘
│ ⌘⏎ Paste  │  ⌘C Copy  │  ⌘⇧V Plain  │  ⌘P Pin  │  ⌘K More │
└─────────────────────────────────────────────────────────────┘
```

### 6.3 Calculator UI

```
┌─────────────────────────────────────────────────────────────┐
│ 100 usd in eur                                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   💱  Currency Conversion                                   │
│                                                             │
│   $100.00 USD  →  €92.47 EUR                               │
│                                                             │
│   Rate: 1 USD = 0.9247 EUR                                 │
│   Updated: 2 hours ago (frankfurter.app)                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
│ ⌘⏎ Copy Formatted  │  ⌘⇧⏎ Copy Raw (92.47)  │  ⌘R Refresh │
└─────────────────────────────────────────────────────────────┘
```

### 6.4 Calendar UI

```
┌─────────────────────────────────────────────────────────────┐
│ 📅 My Schedule                                              │
├─────────────────────────────────────────────────────────────┤
│ TODAY - Friday, January 16                                  │
│ ├─ 🟢 All Day: Team Offsite                                │
│ ├─ 🔵 9:00 AM  Daily Standup (15 min)                      │
│ │              📹 Join Meeting                              │
│ ├─ 🟣 11:00 AM Design Review (1 hr)                        │
│ │              📍 Room 3A                                   │
│ └─ 🔴 2:00 PM  Client Call (30 min)                        │
│                📹 Join Meeting ← Starting in 10 min         │
├─────────────────────────────────────────────────────────────┤
│ TOMORROW - Saturday, January 17                             │
│ └─ 🟢 All Day: Weekend                                     │
└─────────────────────────────────────────────────────────────┘
│ ⏎ Join Meeting  │  ⌘O Open in Calendar  │  ⌘C Copy Details │
└─────────────────────────────────────────────────────────────┘
```

### 6.5 App Uninstaller UI

```
┌─────────────────────────────────────────────────────────────┐
│ 🗑️ Uninstall Slack                                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ⚠️  This will move Slack and related files to Trash      │
│                                                             │
│   📦 Application                                            │
│      /Applications/Slack.app                    512 MB     │
│                                                             │
│   📁 Related Files (Deep Scan)                              │
│   ☑️  ~/Library/Application Support/Slack      128 MB      │
│   ☑️  ~/Library/Preferences/com.tinyspeck...   4 KB        │
│   ☑️  ~/Library/Caches/com.tinyspeck.slack    64 MB       │
│   ☑️  ~/Library/Logs/Slack                    2 MB        │
│   ☑️  ~/Library/Saved Application State/...   1 KB        │
│                                                             │
│   Total space to free: 706 MB                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
│ ⏎ Uninstall  │  ⌘⇧⏎ Keep Related Files  │  ⎋ Cancel       │
└─────────────────────────────────────────────────────────────┘
```

### 6.6 Preferences UI

```
┌─────────────────────────────────────────────────────────────┐
│ ⚙️ Preferences                                               │
├─────────────────────────────────────────────────────────────┤
│ APPEARANCE                                                  │
│ ├─ Theme: [Catppuccin Mocha ▼]                             │
│ │         ○ Latte (Light)                                  │
│ │         ○ Frappé (Dark - Low)                            │
│ │         ○ Macchiato (Dark - Medium)                      │
│ │         ● Mocha (Dark - High)                            │
│ │         ○ Auto (Follow System)                           │
│ ├─ Accent Color:                                           │
│ │   ● ● ● ● ● ● ●  (14 color swatches)                    │
│ └─ Animation: [✓] Enabled (200ms)                          │
├─────────────────────────────────────────────────────────────┤
│ CLIPBOARD                                                   │
│ ├─ History Size: [1000] items                              │
│ ├─ Retention: [30] days                                    │
│ ├─ Store Images: [✓] (Max 10MB)                            │
│ └─ Excluded Apps: [Configure...]                           │
├─────────────────────────────────────────────────────────────┤
│ KEYBOARD SHORTCUTS                                          │
│ ├─ Global Hotkey: [⌘ Space]                                │
│ ├─ Clipboard: [⌘⇧V]                                        │
│ └─ [Configure All Shortcuts...]                            │
└─────────────────────────────────────────────────────────────┘
```

---

## 7. Extension API Contracts

### 7.1 IPC Protocol

Communication between PhotonCast and extensions uses JSON-RPC over stdio:

```rust
/// Host -> Extension messages
#[derive(Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum HostMessage {
    #[serde(rename = "extension.load")]
    Load { preferences: HashMap<String, Value> },
    
    #[serde(rename = "extension.run")]
    Run { command: String, arguments: Option<Value> },
    
    #[serde(rename = "search.query")]
    SearchQuery { query: String },
    
    #[serde(rename = "action.execute")]
    ExecuteAction { action_id: String, item_id: Option<String> },
    
    #[serde(rename = "lifecycle.unload")]
    Unload,
}

/// Extension -> Host messages
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ExtensionMessage {
    #[serde(rename = "ui.render")]
    Render { view: ExtensionView },
    
    #[serde(rename = "ui.loading")]
    Loading { is_loading: bool },
    
    #[serde(rename = "toast.show")]
    ShowToast { options: ToastOptions },
    
    #[serde(rename = "hud.show")]
    ShowHUD { title: String },
    
    #[serde(rename = "navigation.push")]
    Push { view: ExtensionView },
    
    #[serde(rename = "navigation.pop")]
    Pop,
    
    #[serde(rename = "clipboard.copy")]
    CopyToClipboard { content: ClipboardContent },
    
    #[serde(rename = "open.url")]
    OpenURL { url: String },
    
    #[serde(rename = "search.results")]
    SearchResults { items: Vec<SearchResultItem> },
    
    #[serde(rename = "error")]
    Error { message: String, recoverable: bool },
}
```

### 7.2 Extension View Schema

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionView {
    List {
        is_loading: bool,
        search_placeholder: Option<String>,
        items: Vec<ListItem>,
        sections: Vec<ListSection>,
        empty_view: Option<EmptyView>,
    },
    Grid {
        is_loading: bool,
        columns: u8,
        aspect_ratio: String,
        items: Vec<GridItem>,
    },
    Detail {
        markdown: String,
        metadata: Option<Vec<MetadataItem>>,
        is_loading: bool,
    },
    Form {
        fields: Vec<FormField>,
        submit_title: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct ListItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<Icon>,
    pub accessories: Vec<Accessory>,
    pub actions: Vec<Action>,
    pub keywords: Vec<String>,
    pub detail: Option<Box<ExtensionView>>,
}

#[derive(Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub title: String,
    pub icon: Option<Icon>,
    pub shortcut: Option<Shortcut>,
    pub style: ActionStyle,
}

#[derive(Serialize, Deserialize)]
pub struct Shortcut {
    pub modifiers: Vec<Modifier>,
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub enum Modifier {
    Cmd,
    Ctrl,
    Opt,
    Shift,
}
```

### 7.3 API Stability

- Mark unstable APIs with `#[unstable]` attribute
- Semver for published extension API crate
- Warn extension developers about potential breaking changes
- Compatibility checks during extension loading

---

## 8. Security Considerations

### 8.1 Data Protection

| Data Type | Protection |
|-----------|------------|
| Clipboard History | AES-256-GCM encryption with machine-derived key |
| API Keys (preferences) | Stored in macOS Keychain |
| Extension Storage | Isolated per-extension |
| TOML Configs | Plain text (user-editable by design) |

### 8.2 Extension Sandboxing

```rust
pub struct ExtensionSandbox {
    /// Allowed network domains (if network permission granted)
    allowed_domains: Option<Vec<String>>,
    /// File system access scope
    fs_read_paths: Vec<PathBuf>,
    fs_write_paths: Vec<PathBuf>,
    /// Process isolation
    isolated_process: bool,
}

impl ExtensionSandbox {
    pub fn from_permissions(permissions: &ExtensionPermissions) -> Self {
        Self {
            allowed_domains: if permissions.network {
                None  // All domains allowed
            } else {
                Some(vec![])  // No network access
            },
            fs_read_paths: if permissions.filesystem_read {
                vec![dirs::home_dir().unwrap()]  // User directory
            } else {
                vec![]
            },
            fs_write_paths: if permissions.filesystem_write {
                vec![get_extension_data_dir()]  // Extension directory only
            } else {
                vec![]
            },
            isolated_process: true,
        }
    }
}
```

### 8.3 Permission Handling

| Permission | Handling |
|------------|----------|
| Accessibility | Prompt on first use + inline status |
| Calendar | Request on first calendar command |
| Automation | Request when needed |

### 8.4 Sensitive Data Handling

- Never log sensitive clipboard content
- Exclude password managers by default
- Respect NSPasteboard transient flag
- Clear sensitive data from memory after use

---

## 9. Performance Requirements

### 9.1 Latency Targets

| Operation | Target | Maximum |
|-----------|--------|---------|
| Calculator evaluation | < 5ms | 20ms |
| Clipboard history load | < 100ms | 200ms |
| Window resize (excluding animation) | < 50ms | 100ms |
| Calendar events load | < 500ms | 1000ms |
| Extension load | < 50ms | 100ms |
| Quick Links search | < 10ms | 50ms |
| App list load | < 100ms | 200ms |
| Currency rate fetch | < 2000ms | 5000ms |

### 9.2 Memory Targets

| Component | Target | Maximum |
|-----------|--------|---------|
| Base application | < 50MB | 100MB |
| Clipboard history (1000 items) | < 100MB | 200MB |
| Per extension | < 20MB | 50MB |
| Image thumbnails cache | < 50MB | 100MB |

### 9.3 Animation Performance

- Target: 120 FPS rendering
- Animation duration: 200ms
- Respect "Reduce Motion" accessibility setting

### 9.4 Benchmarks to Implement

```rust
// benches/phase2_bench.rs

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_calculator(c: &mut Criterion) {
    let calculator = Calculator::new();
    
    c.bench_function("calc_basic_math", |b| {
        b.iter(|| calculator.evaluate("1 + 2 * 3"))
    });
    
    c.bench_function("calc_currency_conversion", |b| {
        b.iter(|| calculator.evaluate("100 usd in eur"))
    });
    
    c.bench_function("calc_unit_conversion", |b| {
        b.iter(|| calculator.evaluate("100 km to miles"))
    });
}

fn bench_clipboard(c: &mut Criterion) {
    let storage = ClipboardStorage::new_test();
    
    c.bench_function("clipboard_load_1000", |b| {
        b.iter(|| storage.load_recent(1000))
    });
    
    c.bench_function("clipboard_search", |b| {
        b.iter(|| storage.search("test query"))
    });
}

fn bench_extension(c: &mut Criterion) {
    c.bench_function("extension_load", |b| {
        b.iter(|| ExtensionHost::load("test-extension"))
    });
}

criterion_group!(benches, bench_calculator, bench_clipboard, bench_extension);
criterion_main!(benches);
```

---

## 10. Testing Strategy

### 10.1 Coverage Targets

| Metric | Target |
|--------|--------|
| Overall test coverage | 80% |
| Critical paths | 95% |
| Public APIs | 100% |

### 10.2 Test Types

**Unit Tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn calculator_evaluates_basic_math() {
        let calc = Calculator::new();
        let result = calc.evaluate("2 + 3 * 4").unwrap();
        assert_eq!(result.raw_value, 14.0);
    }
    
    #[test]
    fn calculator_handles_currency_conversion() {
        let mut calc = Calculator::new();
        calc.set_rate("USD", "EUR", 0.92);
        
        let result = calc.evaluate("100 usd in eur").unwrap();
        assert!((result.raw_value - 92.0).abs() < 0.01);
    }
    
    #[test]
    fn clipboard_excludes_password_managers() {
        let monitor = ClipboardMonitor::new_test();
        let item = ClipboardItem {
            source_bundle_id: Some("com.1password.1password".to_string()),
            ..Default::default()
        };
        
        assert!(monitor.should_exclude(&item));
    }
}
```

**Integration Tests:**

```rust
// tests/integration/clipboard_test.rs

#[tokio::test]
async fn clipboard_full_workflow() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = ClipboardStorage::new(temp_dir.path()).await.unwrap();
    
    // Store item
    let item = ClipboardItem::text("Hello, World!");
    storage.store(item).await.unwrap();
    
    // Retrieve
    let items = storage.load_recent(10).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].text_content(), Some("Hello, World!"));
    
    // Search
    let results = storage.search("Hello").await.unwrap();
    assert_eq!(results.len(), 1);
    
    // Pin
    storage.pin(items[0].id).await.unwrap();
    let pinned = storage.load_pinned().await.unwrap();
    assert_eq!(pinned.len(), 1);
}

// tests/integration/extension_test.rs

#[tokio::test]
async fn extension_lifecycle() {
    let host = ExtensionHost::new_test();
    
    // Load extension
    let ext = host.load("test-extension").await.unwrap();
    assert!(ext.is_loaded());
    
    // Run command
    let result = host.run_command(&ext, "main", None).await.unwrap();
    assert!(matches!(result, ExtensionView::List { .. }));
    
    // Unload
    host.unload(&ext).await.unwrap();
    assert!(!ext.is_loaded());
}
```

**UI Tests (Critical Components):**

```rust
// tests/ui/search_bar_test.rs

#[gpui::test]
async fn search_bar_updates_on_input(cx: &mut TestAppContext) {
    let view = cx.new_view(|_| SearchBar::new());
    
    cx.simulate_keystrokes("f i r e f o x");
    
    view.read(cx, |view, _| {
        assert_eq!(view.query(), "firefox");
    });
}

#[gpui::test]
async fn results_list_navigates_with_arrows(cx: &mut TestAppContext) {
    let view = cx.new_view(|_| ResultsList::new_with_items(vec![
        item("First"),
        item("Second"),
        item("Third"),
    ]));
    
    cx.simulate_keystroke("down");
    view.read(cx, |view, _| assert_eq!(view.selected_index(), 1));
    
    cx.simulate_keystroke("down");
    view.read(cx, |view, _| assert_eq!(view.selected_index(), 2));
    
    cx.simulate_keystroke("up");
    view.read(cx, |view, _| assert_eq!(view.selected_index(), 1));
}
```

### 10.3 Test Matrix

| Feature | Unit | Integration | UI | Benchmark |
|---------|------|-------------|-----|-----------|
| Clipboard | ✓ | ✓ | ✓ | ✓ |
| Calculator | ✓ | ✓ | - | ✓ |
| Window Management | ✓ | ✓ | - | - |
| Calendar | ✓ | ✓ | - | - |
| App Management | ✓ | ✓ | - | - |
| Quick Links | ✓ | ✓ | - | - |
| Sleep Timer | ✓ | ✓ | - | - |
| Preferences | ✓ | ✓ | ✓ | - |
| Extensions | ✓ | ✓ | ✓ | ✓ |
| Custom Commands | ✓ | ✓ | - | - |

---

## 11. Dependencies and Crates

### 11.1 Core Dependencies

```toml
[dependencies]
# GUI Framework
gpui = "0.1"
gpui-component = "0.1"

# Async Runtime
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs", "process", "sync", "time"] }
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Storage
rusqlite = { version = "0.31", features = ["bundled"] }
directories = "5.0"

# Error Handling
thiserror = "2.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
regex = "1"
once_cell = "1"
parking_lot = "0.12"
```

### 11.2 Feature-Specific Crates

**Calculator:**

```toml
# Math expression parsing
evalexpr = "11"

# High-precision decimals for currency
rust_decimal = "1"

# Date/time with timezones
chrono-tz = "0.8"

# Natural language date parsing (evaluate both)
dateparser = "0.2"
# OR
chrono-english = "0.1"

# HTTP for rate fetching
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
```

**Clipboard:**

```toml
# Encryption
aes-gcm = "0.10"
argon2 = "0.5"
rand = "0.8"

# Image processing
image = { version = "0.24", default-features = false, features = ["png", "jpeg"] }
```

**Window Management:**

```toml
# Accessibility APIs
accessibility = "0.1"

# Core Graphics bindings
core-graphics = "0.23"
```

**Calendar:**

```toml
# EventKit bindings
objc2-event-kit = "0.2"
```

**macOS Platform:**

```toml
# Objective-C interop
objc2 = "0.5"
objc2-foundation = "0.2"
objc2-app-kit = "0.2"

# Core Foundation
core-foundation = "0.10"

# Plist parsing
plist = "1"

# Security (Keychain)
security-framework = "2"
```

### 11.3 Crates to Evaluate

| Crate | Purpose | Alternative |
|-------|---------|-------------|
| `evalexpr` | Math parsing | `meval` |
| `dateparser` | NL date parsing | `chrono-english` |
| `notify` | File watching | - |
| `global-hotkey` | Global hotkeys | Custom CGEvent |
| `icns` | Icon loading | - |
| `nucleo` | Fuzzy matching | `fuzzy-matcher` |

### 11.4 Dev Dependencies

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
proptest = "1"
tempfile = "3"
tokio-test = "0.4"
```

---

## 12. Risks and Mitigations

### 12.1 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| GPUI API changes | Medium | High | Pin GPUI version, monitor upstream |
| macOS API deprecation | Low | Medium | Use stable APIs, plan migration path |
| Extension sandbox bypass | Low | High | Security audit, minimal permissions |
| Performance regression | Medium | Medium | Continuous benchmarking in CI |
| Calendar permission issues | Medium | Medium | Graceful degradation, clear error messages |

### 12.2 Schedule Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Extension system complexity | High | High | Start early, MVP approach |
| Window management edge cases | Medium | Medium | Focus on common layouts first |
| Browser import variations | Medium | Low | Support main browsers, document limitations |

### 12.3 User Experience Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Permission fatigue | Medium | Medium | Request just-in-time, explain benefits |
| Feature discoverability | Medium | Medium | Onboarding flow, keyboard shortcut hints |
| Migration from Raycast | Low | Medium | Import tools, familiar shortcuts |

---

## 13. Success Metrics

### 13.1 Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test Coverage | 80% | `cargo llvm-cov` |
| Bug Count (Open) | < 20 | GitHub Issues |
| Crash Rate | < 0.1% | Crash reports |
| Performance Regression | 0% | CI benchmarks |

### 13.2 Feature Completion

| Feature | Sprint | Exit Criteria |
|---------|--------|---------------|
| Clipboard History | 4 | All content types, encryption, search |
| Calculator | 4 | Math, currency, units, datetime |
| Window Management | 5 | All layouts, cycling, multi-monitor |
| Quick Links | 5 | CRUD, import, dynamic URLs |
| Calendar | 5 | Display, conference detection, join |
| App Management | 5 | Uninstall, force quit, app sleep |
| Sleep Timer | 5 | Schedule, persist, cancel |
| Preferences | 5 | All settings, themes, shortcuts |
| Extension System | 6 | Load, sandbox, hot-reload |
| Custom Commands | 6 | Execute, stream output |
| First-Party Extensions | 6 | 3 bundled extensions |

### 13.3 User Metrics (Post-Launch)

| Metric | Target | Timeframe |
|--------|--------|-----------|
| Daily Active Users | 500+ | 3 months |
| GitHub Stars | 1,000+ | 3 months |
| Native Extensions | 5+ | 3 months |
| User-Reported Bugs | < 20 open | Ongoing |
| NPS Score | 50+ | 3 months |

### 13.4 Acceptance Criteria Summary

- [ ] Clipboard captures all copy events across supported content types
- [ ] Calculator evaluates in under 5ms
- [ ] Clipboard history is searchable with full-text search
- [ ] Currency rates updated every 6 hours with offline cache
- [ ] Windows resize smoothly with 200ms animation
- [ ] Multi-monitor detection and movement works correctly
- [ ] Quick links open instantly with favicon display
- [ ] Calendar events load in under 500ms
- [ ] Enter joins meeting for events with conference links
- [ ] App uninstall cleans up 90%+ of related files
- [ ] Native extensions load in under 50ms
- [ ] Extension API is documented with examples
- [ ] Custom commands execute with real-time output streaming
- [ ] All 4 Catppuccin themes render correctly
- [ ] 80% test coverage achieved
- [ ] Integration tests pass for all features

---

## Appendix A: Keyboard Shortcuts Reference

### Global

| Shortcut | Action |
|----------|--------|
| `Cmd+Space` | Open PhotonCast (default, configurable) |
| `Cmd+Shift+V` | Open Clipboard History |

### Launcher

| Shortcut | Action |
|----------|--------|
| `↑` / `↓` | Navigate results |
| `Enter` | Primary action |
| `Cmd+Enter` | Secondary action |
| `Cmd+K` | Open action panel |
| `Escape` | Close / Go back |
| `Cmd+C` | Copy selected |
| `Cmd+O` | Open in default app |
| `Cmd+Shift+O` | Reveal in Finder |
| `Cmd+Backspace` | Delete / Trash |
| `Cmd+R` | Refresh |
| `Tab` | Next section |
| `Shift+Tab` | Previous section |

### Clipboard History

| Shortcut | Action |
|----------|--------|
| `Enter` | Paste (default) |
| `Cmd+C` | Copy without pasting |
| `Cmd+Shift+V` | Paste as plain text |
| `Cmd+Opt+V` | Paste and don't save |
| `Cmd+P` | Pin / Unpin |
| `Cmd+Backspace` | Delete item |
| `Cmd+Shift+Backspace` | Clear all history |

### Calculator

| Shortcut | Action |
|----------|--------|
| `Enter` | Copy formatted result |
| `Cmd+Enter` | Copy raw number |
| `Cmd+R` | Refresh rates |

---

## Appendix B: Configuration Reference

See Section 5.2 for complete configuration schema.

---

## Appendix C: Extension API Reference

See Section 7 for complete API documentation.

---

*End of Specification Document*
