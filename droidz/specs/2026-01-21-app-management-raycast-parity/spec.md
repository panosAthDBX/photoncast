# App Management - Raycast Parity Specification

> **Version:** 1.0.0  
> **Date:** 2026-01-21  
> **Status:** Draft  
> **Replaces:** Section 3.2.4 of Phase 2 spec

---

## Table of Contents

1. [Overview](#1-overview)
2. [User Stories](#2-user-stories)
3. [Feature Specifications](#3-feature-specifications)
4. [UI/UX Specifications](#4-uiux-specifications)
5. [Data Models](#5-data-models)
6. [Technical Implementation](#6-technical-implementation)
7. [Testing Requirements](#7-testing-requirements)

---

## 1. Overview

### 1.1 Goal

Implement app management features with **100% Raycast parity**, allowing users to manage installed applications directly from the launcher with a comprehensive action panel.

### 1.2 Raycast Feature Mapping

| Raycast Feature | PhotonCast Implementation | Priority |
|-----------------|---------------------------|----------|
| App search in root | ✅ Already implemented | - |
| Running app indicator | 🆕 New | P0 |
| Action Panel (⌘K) | 🆕 New actions needed | P0 |
| Show in Finder | 🆕 New | P0 |
| Copy Bundle ID | 🆕 New | P0 |
| Copy Path | 🆕 New | P0 |
| Quit App | 🆕 New | P0 |
| Force Quit App | 🆕 New | P0 |
| Hide App | 🆕 New | P1 |
| Uninstall with cleanup | ✅ Partially implemented | P0 |
| Auto Quit | 🆕 New | P1 |
| Auto Quit indicator | 🆕 New | P1 |
| Show Info (Get Info) | 🆕 New | P2 |

---

## 2. User Stories

### 2.1 Core App Actions

**US-1:** As a user, when I search for an app in the launcher, I want to see if it's currently running so I can decide whether to open or manage it.

**US-2:** As a user, I want to press ⌘K on a selected app to see all available actions (Open, Show in Finder, Copy Path, Copy Bundle ID, Quit, Force Quit, Hide, Uninstall).

**US-3:** As a user, I want to quit or force quit an app directly from the launcher without switching to that app first.

**US-4:** As a user, I want to quickly reveal an app's location in Finder to access its bundle or related files.

**US-5:** As a user, I want to copy an app's bundle ID or path to use in scripts, terminal commands, or configuration files.

### 2.2 Auto Quit

**US-6:** As a user, I want to enable Auto Quit for distracting apps (like Slack, Messages) so they automatically close after a period of inactivity.

**US-7:** As a user, I want to see a visual indicator on apps that have Auto Quit enabled, so I know which apps will close automatically.

**US-8:** As a user, I want to configure the inactivity timeout per app (default: 3 minutes).

### 2.3 Uninstall

**US-9:** As a user, I want to uninstall an app and all its related files (preferences, caches, logs, etc.) in one action.

**US-10:** As a user, I want to preview what files will be removed before confirming the uninstall.

**US-11:** As a user, I want the option to keep related files when uninstalling (for potential reinstall).

---

## 3. Feature Specifications

### 3.1 Running App Indicator

**Behavior:**
- Show a small colored dot indicator on apps that are currently running
- Dot color: Green (matches Raycast)
- Position: Bottom-right of app icon
- Size: 8px diameter

**Detection:**
```rust
pub fn is_app_running(bundle_id: &str) -> bool {
    // Use NSWorkspace.runningApplications
    // Match by bundle identifier
}

pub fn get_running_apps() -> Vec<RunningApp> {
    // Return list of all running applications
    // Include: bundle_id, pid, is_active, is_hidden
}
```

### 3.2 Action Panel Actions

When user presses ⌘K on a selected app, show these actions:

#### Always Available Actions

| Action | Shortcut | Description |
|--------|----------|-------------|
| Open | ↵ (Enter) | Launch/activate the app |
| Show in Finder | ⌘⇧F | Reveal app bundle in Finder |
| Copy Path | ⌘⇧C | Copy full path to clipboard |
| Copy Bundle ID | ⌘⇧B | Copy bundle identifier |
| Uninstall | ⌘⌫ | Uninstall with cleanup preview |

#### Running App Actions (only shown if app is running)

| Action | Shortcut | Description |
|--------|----------|-------------|
| Quit | ⌘Q | Graceful quit (sends quit event) |
| Force Quit | ⌘⌥Q | Immediate termination (SIGKILL) |
| Hide | ⌘H | Hide app windows |

#### Auto Quit Actions

| Action | Shortcut | Description |
|--------|----------|-------------|
| Enable Auto Quit | - | Enable auto quit for this app |
| Disable Auto Quit | - | Disable auto quit (if enabled) |
| Configure Auto Quit | - | Open settings for this app |

### 3.3 Show in Finder

**Behavior:**
```rust
pub fn reveal_in_finder(path: &Path) -> Result<()> {
    // Use NSWorkspace.selectFile:inFileViewerRootedAtPath:
    // Or: open -R "/Applications/App.app"
}
```

**Implementation:**
- Select the app bundle in Finder
- Bring Finder to foreground
- Works for apps in /Applications, ~/Applications, and other locations

### 3.4 Copy Actions

**Copy Path:**
- Copy full POSIX path: `/Applications/Slack.app`
- Show toast: "Path copied to clipboard"

**Copy Bundle ID:**
- Copy bundle identifier: `com.tinyspeck.slackmacgap`
- Show toast: "Bundle ID copied to clipboard"

### 3.5 Quit and Force Quit

**Quit (Graceful):**
```rust
pub async fn quit_app(bundle_id: &str) -> Result<bool> {
    // Send NSApplicationTerminate event
    // Wait up to 5 seconds for app to quit
    // Return true if app quit successfully
}
```

**Force Quit:**
```rust
pub fn force_quit_app(pid: i32) -> Result<()> {
    // Send SIGKILL to process
    // kill -9 <pid>
}
```

**Confirmation:**
- Quit: No confirmation needed
- Force Quit: Show warning "Force quitting may cause unsaved data loss. Continue?"
- Skip confirmation if app is "Not Responding"

**Not Responding Detection:**
```rust
pub fn is_app_responding(pid: i32) -> bool {
    // Use NSRunningApplication.isTerminated or 
    // Check if app responds to Apple Events within 2 seconds
}
```

### 3.6 Hide App

**Behavior:**
```rust
pub fn hide_app(bundle_id: &str) -> Result<()> {
    // Use NSRunningApplication.hide()
    // Or: osascript -e 'tell app "AppName" to set visible to false'
}
```

### 3.7 Auto Quit Feature

**Overview:**
Automatically quit applications after a configurable period of inactivity. Designed for "distracting" apps like messaging, social media, and calendar apps.

**Default Behavior:**
- Disabled by default (user must explicitly enable per app)
- Default inactivity timeout: 3 minutes
- Inactivity = no window is focused for the app

**Visual Indicator:**
- Small orange dot under app icon (in addition to green running dot)
- Tooltip: "Auto Quit enabled (3 min)"

**Configuration:**

```rust
pub struct AutoQuitConfig {
    /// Apps with auto quit enabled
    pub apps: HashMap<BundleId, AutoQuitAppConfig>,
}

pub struct AutoQuitAppConfig {
    /// Whether auto quit is enabled
    pub enabled: bool,
    /// Inactivity timeout in minutes (default: 3)
    pub timeout_minutes: u32,
    /// Last activity timestamp
    pub last_active: Option<DateTime<Utc>>,
}
```

**Inactivity Tracking:**
```rust
pub struct AutoQuitManager {
    config: AutoQuitConfig,
    /// Tracks when each app was last active (had focus)
    activity_tracker: HashMap<BundleId, Instant>,
}

impl AutoQuitManager {
    /// Called when any app gains focus
    pub fn on_app_activated(&mut self, bundle_id: &str) {
        self.activity_tracker.insert(bundle_id.into(), Instant::now());
    }
    
    /// Called periodically (every 30 seconds) to check for inactive apps
    pub async fn check_and_quit_inactive(&mut self) -> Vec<String> {
        let mut quit_apps = vec![];
        for (bundle_id, config) in &self.config.apps {
            if !config.enabled { continue; }
            if let Some(last_active) = self.activity_tracker.get(bundle_id) {
                let inactive_duration = last_active.elapsed();
                if inactive_duration > Duration::from_secs(config.timeout_minutes as u64 * 60) {
                    if quit_app(bundle_id).await.is_ok() {
                        quit_apps.push(bundle_id.clone());
                    }
                }
            }
        }
        quit_apps
    }
}
```

**Suggested Apps (for quick setup):**
- Messaging: Slack, Discord, Messages, WhatsApp, Telegram
- Calendar: Calendar, Fantastical, Notion Calendar
- Social: Twitter/X, Bluesky, Mastodon, Facebook
- Email: Mail, Spark, Airmail (use with caution)

### 3.8 Uninstall with Cleanup

**Preview Phase:**
Show user exactly what will be deleted:

```
┌─────────────────────────────────────────────────────────────┐
│ 🗑️ Uninstall Slack                                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ⚠️  This will move Slack and related files to Trash      │
│                                                             │
│   ☑️ Application                                            │
│      /Applications/Slack.app                     420 MB    │
│                                                             │
│   ☑️ Application Support                                    │
│      ~/Library/Application Support/Slack         285 MB    │
│                                                             │
│   ☑️ Preferences                                            │
│      ~/Library/Preferences/com.tinyspeck...plist   4 KB    │
│                                                             │
│   ☑️ Caches                                                 │
│      ~/Library/Caches/com.tinyspeck.slackmacgap  156 MB    │
│                                                             │
│   ☑️ Logs                                                   │
│      ~/Library/Logs/Slack                          2 MB    │
│                                                             │
│   ☑️ Saved State                                            │
│      ~/Library/Saved Application State/...         1 KB    │
│                                                             │
│   ───────────────────────────────────────────────────────  │
│   Total space to free: 863 MB                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
│ ⏎ Uninstall  │  ⌘⇧⏎ Keep Related Files  │  ⎋ Cancel       │
└─────────────────────────────────────────────────────────────┘
```

**Related File Categories:**

```rust
pub enum RelatedFileCategory {
    /// ~/Library/Application Support/<App or BundleID>
    ApplicationSupport,
    /// ~/Library/Preferences/<BundleID>.plist
    Preferences,
    /// ~/Library/Caches/<BundleID>
    Caches,
    /// ~/Library/Logs/<App>
    Logs,
    /// ~/Library/Saved Application State/<BundleID>.savedState
    SavedState,
    /// ~/Library/Containers/<BundleID> (sandboxed apps)
    Containers,
    /// ~/Library/Group Containers/<GroupID> (app groups)
    GroupContainers,
    /// ~/Library/Cookies/<BundleID>.binarycookies
    Cookies,
    /// ~/Library/WebKit/<BundleID>
    WebKit,
    /// ~/Library/HTTPStorages/<BundleID>
    HTTPStorages,
}
```

**Search Paths:**
```rust
const RELATED_FILE_SEARCH_PATHS: &[&str] = &[
    "~/Library/Application Support/",
    "~/Library/Preferences/",
    "~/Library/Caches/",
    "~/Library/Logs/",
    "~/Library/Saved Application State/",
    "~/Library/Containers/",
    "~/Library/Group Containers/",
    "~/Library/Cookies/",
    "~/Library/WebKit/",
    "~/Library/HTTPStorages/",
    "/Library/Application Support/",
    "/Library/Preferences/",
    "/Library/Caches/",
    "/Library/Logs/",
];
```

**Matching Strategy (Conservative):**
- Exact bundle ID match (e.g., `com.tinyspeck.slackmacgap`)
- Exact app name match (e.g., `Slack`)
- Do NOT match partial strings to avoid false positives

**Safety:**
- System apps protected (`/System/Applications/*`)
- Apple apps protected (bundle ID starts with `com.apple.`)
- Move to Trash (not permanent delete) - allows recovery
- Require admin password for apps in `/Applications`

---

## 4. UI/UX Specifications

### 4.1 App Result Item

```
┌─────────────────────────────────────────────────────────────┐
│ [Icon]  Slack                                          ⌘1  │
│   🟢    Messaging app for teams                 Running    │
│   🟠                                                        │
└─────────────────────────────────────────────────────────────┘

Legend:
🟢 = Running indicator (green dot, bottom-right of icon)
🟠 = Auto Quit enabled indicator (orange dot, below green dot)
```

### 4.2 Action Panel for Apps

```
┌─────────────────────────────────────────────────────────────┐
│ Actions for Slack                                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   Primary                                                   │
│   ─────────────────────────────────────────────────────    │
│   ▸ Open                                              ↵    │
│   ▸ Quit                                             ⌘Q    │
│   ▸ Force Quit                                      ⌘⌥Q    │
│   ▸ Hide                                             ⌘H    │
│                                                             │
│   Info                                                      │
│   ─────────────────────────────────────────────────────    │
│   ▸ Show in Finder                                  ⌘⇧F    │
│   ▸ Copy Path                                       ⌘⇧C    │
│   ▸ Copy Bundle ID                                  ⌘⇧B    │
│   ▸ Get Info                                         ⌘I    │
│                                                             │
│   Auto Quit                                                 │
│   ─────────────────────────────────────────────────────    │
│   ▸ Enable Auto Quit                                       │
│                                                             │
│   Danger Zone                                               │
│   ─────────────────────────────────────────────────────    │
│   ▸ Uninstall...                                    ⌘⌫    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 4.3 Auto Quit Settings (per app)

Accessed via Preferences → Extensions → Applications → [App Name]

```
┌─────────────────────────────────────────────────────────────┐
│ Slack - Auto Quit Settings                                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   Auto Quit                                                 │
│   ─────────────────────────────────────────────────────    │
│   [✓] Automatically quit after inactivity                  │
│                                                             │
│   Inactivity Timeout                                        │
│   ┌─────────────────────────────────────────────────────┐  │
│   │ 3 minutes                                       [▼] │  │
│   └─────────────────────────────────────────────────────┘  │
│   Options: 1, 2, 3, 5, 10, 15, 30 minutes                  │
│                                                             │
│   [Disable Auto Quit]                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 4.4 Manage Auto Quits Command

A dedicated command to view/manage all apps with Auto Quit enabled:

```
┌─────────────────────────────────────────────────────────────┐
│ 🔍 Manage Auto Quits                                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   Apps with Auto Quit Enabled                               │
│   ─────────────────────────────────────────────────────    │
│   [Slack icon]    Slack              3 min    [Disable]    │
│   [Discord icon]  Discord            3 min    [Disable]    │
│   [Messages icon] Messages           5 min    [Disable]    │
│   [Calendar icon] Calendar          10 min    [Disable]    │
│                                                             │
│   ─────────────────────────────────────────────────────    │
│   [+ Add App]                                               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. Data Models

### 5.1 Application Model (Extended)

```rust
pub struct Application {
    pub name: String,
    pub path: PathBuf,
    pub bundle_id: String,
    pub icon: Option<IconData>,
    pub version: Option<String>,
    pub is_system_app: bool,
}

pub struct RunningApplication {
    pub app: Application,
    pub pid: i32,
    pub is_active: bool,
    pub is_hidden: bool,
    pub is_responding: bool,
    pub launch_time: DateTime<Utc>,
}

pub struct ApplicationWithState {
    pub app: Application,
    pub running: Option<RunningApplication>,
    pub auto_quit_enabled: bool,
    pub auto_quit_timeout_minutes: Option<u32>,
}
```

### 5.2 Auto Quit Storage

Store in `~/.config/photoncast/auto_quit.toml`:

```toml
[apps]
[apps."com.tinyspeck.slackmacgap"]
enabled = true
timeout_minutes = 3

[apps."com.hnc.Discord"]
enabled = true
timeout_minutes = 3

[apps."com.apple.iCal"]
enabled = true
timeout_minutes = 10
```

### 5.3 Uninstall Preview

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
    pub size_formatted: String,
    pub category: RelatedFileCategory,
    pub selected: bool, // User can deselect individual items
}
```

---

## 6. Technical Implementation

### 6.1 New Crate Structure

Extend existing `photoncast-apps` crate:

```
crates/photoncast-apps/
├── src/
│   ├── lib.rs
│   ├── models.rs         # Application, RunningApplication
│   ├── scanner.rs        # App discovery (existing)
│   ├── bundle.rs         # Bundle parsing (existing)
│   ├── process.rs        # Running app detection, quit, force quit
│   ├── uninstaller.rs    # Uninstall with cleanup (existing, enhance)
│   ├── auto_quit.rs      # NEW: Auto quit manager
│   ├── actions.rs        # NEW: Show in Finder, Copy, Hide
│   └── commands.rs       # Search provider commands
```

### 6.2 macOS APIs Required

| Feature | API |
|---------|-----|
| Running apps | `NSWorkspace.runningApplications` |
| Quit app | `NSRunningApplication.terminate()` |
| Force quit | `kill(pid, SIGKILL)` or `NSRunningApplication.forceTerminate()` |
| Hide app | `NSRunningApplication.hide()` |
| Is responding | `NSRunningApplication.isFinishedLaunching` + Apple Events timeout |
| Show in Finder | `NSWorkspace.selectFile:inFileViewerRootedAtPath:` |
| App activation | `NSWorkspace.didActivateApplicationNotification` |

### 6.3 Action Panel Integration

Add app-specific actions to the existing action panel system:

```rust
impl LauncherWindow {
    fn get_app_actions(&self, app: &ApplicationWithState) -> Vec<ActionItem> {
        let mut actions = vec![
            ActionItem::new("Open", "↵", ActionKind::Open),
            ActionItem::new("Show in Finder", "⌘⇧F", ActionKind::ShowInFinder),
            ActionItem::new("Copy Path", "⌘⇧C", ActionKind::CopyPath),
            ActionItem::new("Copy Bundle ID", "⌘⇧B", ActionKind::CopyBundleId),
        ];
        
        if let Some(running) = &app.running {
            actions.push(ActionItem::new("Quit", "⌘Q", ActionKind::Quit));
            actions.push(ActionItem::new("Force Quit", "⌘⌥Q", ActionKind::ForceQuit));
            actions.push(ActionItem::new("Hide", "⌘H", ActionKind::Hide));
        }
        
        if app.auto_quit_enabled {
            actions.push(ActionItem::new("Disable Auto Quit", "", ActionKind::DisableAutoQuit));
        } else {
            actions.push(ActionItem::new("Enable Auto Quit", "", ActionKind::EnableAutoQuit));
        }
        
        if !app.app.is_system_app {
            actions.push(ActionItem::new("Uninstall...", "⌘⌫", ActionKind::Uninstall));
        }
        
        actions
    }
}
```

---

## 7. Testing Requirements

### 7.1 Unit Tests

| Test | Description |
|------|-------------|
| `test_running_app_detection` | Verify running apps are correctly detected |
| `test_quit_app` | Verify graceful quit works |
| `test_force_quit_app` | Verify force quit terminates process |
| `test_hide_app` | Verify app windows are hidden |
| `test_show_in_finder` | Verify Finder reveals correct path |
| `test_copy_bundle_id` | Verify correct bundle ID is copied |
| `test_copy_path` | Verify correct path is copied |
| `test_auto_quit_tracking` | Verify activity tracking works |
| `test_auto_quit_timeout` | Verify apps quit after timeout |
| `test_uninstall_preview` | Verify all related files are found |
| `test_uninstall_execution` | Verify files are moved to trash |
| `test_system_app_protection` | Verify system apps cannot be uninstalled |

### 7.2 Integration Tests

| Test | Description |
|------|-------------|
| `test_app_search_shows_running_indicator` | UI shows running dot |
| `test_action_panel_shows_running_actions` | Quit/Force Quit shown for running apps |
| `test_auto_quit_persists_across_restarts` | Config survives app restart |
| `test_uninstall_cleans_all_files` | All related files removed |

---

## Appendix A: Raycast Feature Comparison

| Feature | Raycast | PhotonCast (After) |
|---------|---------|-------------------|
| App search | ✅ | ✅ |
| Running indicator | ✅ Green dot | ✅ Green dot |
| Auto Quit indicator | ✅ Orange dot | ✅ Orange dot |
| Open | ✅ | ✅ |
| Quit | ✅ | ✅ |
| Force Quit | ✅ | ✅ |
| Hide | ✅ | ✅ |
| Show in Finder | ✅ | ✅ |
| Copy Path | ✅ | ✅ |
| Copy Bundle ID | ✅ | ✅ |
| Get Info | ✅ | ✅ |
| Auto Quit | ✅ | ✅ |
| Manage Auto Quits | ✅ | ✅ |
| Uninstall | ✅ | ✅ |
| Deep scan cleanup | ✅ | ✅ |
| Smart Setup (AI) | ✅ PRO | ❌ Not planned |

---

## Appendix B: Migration from Current Spec

### Removed Features
- **App Sleep** (renamed to Auto Quit for Raycast parity)

### Added Features
- Running app indicator (green dot)
- Auto Quit indicator (orange dot)
- Hide app action
- Copy Bundle ID action
- Copy Path action
- Show in Finder action
- Get Info action
- Manage Auto Quits command
- Not Responding detection

### Enhanced Features
- Uninstall: Added more file categories (Cookies, WebKit, HTTPStorages, Group Containers)
- Uninstall: User can deselect individual files
- Action panel: Grouped into sections (Primary, Info, Auto Quit, Danger Zone)
