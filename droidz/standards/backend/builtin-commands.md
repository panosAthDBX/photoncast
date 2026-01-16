# PhotonCast Built-in Commands

> Complete reference for all native commands and features

## Overview

PhotonCast includes a comprehensive set of built-in commands that don't require extensions. These are implemented in native Rust for maximum performance and deep system integration.

---

## Calculator

A natural language calculator supporting math, conversions, dates, and timezones.

### Features

| Category | Examples | Output |
|----------|----------|--------|
| **Basic Math** | `60 + 74` | `134` |
| | `4 power 6` | `4,096` |
| | `32% of 500` | `160` |
| | `sqrt(144)` | `12` |
| | `19m + 47%` | `27.93 m` |
| **Currency** | `100 usd in eur` | `€92.50` |
| | `45 jpy to inr` | `₹31.23` |
| | `0.5 btc in usd` | `$21,500` |
| | `8 dollars/hour in gbp` | `£6.40/hour` |
| **Units** | `23C to F` | `73.4 °F` |
| | `29 inches to cm` | `73.66 cm` |
| | `4 feet to meters` | `1.22 m` |
| | `3 teaspoon in ml` | `14.79 ml` |
| | `100 mph in km/h` | `160.93 km/h` |
| | `5 GB to MB` | `5,120 MB` |
| **Time/Date** | `monday in 3 weeks` | `Feb 3, 2025` |
| | `days until dec 25` | `243 days` |
| | `35 days ago` | `Dec 10, 2024` |
| | `time in dubai` | `17:27 GST` |
| | `5pm ldn in sf` | `09:00 PST` |
| | `2pm est to pst` | `11:00 PST` |

### Implementation

```rust
use std::collections::HashMap;

pub struct Calculator {
    currency_rates: HashMap<String, f64>,
    last_rate_update: Instant,
}

impl Calculator {
    /// Parse and evaluate natural language math expression
    pub fn evaluate(&self, input: &str) -> Result<CalculatorResult> {
        // 1. Tokenize input
        let tokens = self.tokenize(input)?;
        
        // 2. Detect expression type
        let expr_type = self.detect_type(&tokens)?;
        
        // 3. Evaluate based on type
        match expr_type {
            ExprType::Math => self.eval_math(&tokens),
            ExprType::Currency => self.eval_currency(&tokens),
            ExprType::Unit => self.eval_unit(&tokens),
            ExprType::DateTime => self.eval_datetime(&tokens),
            ExprType::Timezone => self.eval_timezone(&tokens),
        }
    }
    
    /// Update currency rates in background
    pub async fn update_rates(&mut self) -> Result<()> {
        let rates = fetch_exchange_rates().await?;
        self.currency_rates = rates;
        self.last_rate_update = Instant::now();
        Ok(())
    }
}

#[derive(Debug)]
pub struct CalculatorResult {
    pub value: String,
    pub formatted: String,
    pub unit: Option<String>,
    pub raw_value: f64,
}
```

### Supported Operations

**Math Functions:**
- Basic: `+`, `-`, `*`, `/`, `^`, `%`
- Functions: `sqrt`, `sin`, `cos`, `tan`, `log`, `ln`, `abs`, `floor`, `ceil`, `round`
- Constants: `pi`, `e`
- Parentheses for grouping

**Currency Codes:**
- Major: USD, EUR, GBP, JPY, CNY, CAD, AUD, CHF
- Crypto: BTC, ETH, USDT, BNB, XRP, ADA, DOGE, SOL
- 150+ fiat currencies supported

**Unit Categories:**
- Length: mm, cm, m, km, in, ft, yd, mi
- Weight: mg, g, kg, oz, lb, ton
- Volume: ml, l, tsp, tbsp, cup, pt, qt, gal
- Temperature: C, F, K
- Data: B, KB, MB, GB, TB, PB
- Speed: m/s, km/h, mph, knots

---

## System Commands

Control macOS without touching the mouse.

### Available Commands

| Command | Alias | Description | Shortcut |
|---------|-------|-------------|----------|
| `Lock Screen` | `lock` | Lock the Mac screen | `Cmd+Ctrl+Q` |
| `Sleep` | `sleep` | Put Mac to sleep | - |
| `Sleep Displays` | `sleep display` | Turn off displays only | - |
| `Restart` | `restart`, `reboot` | Restart the Mac | - |
| `Shut Down` | `shutdown`, `power off` | Shut down the Mac | - |
| `Log Out` | `logout`, `sign out` | Log out current user | - |
| `Empty Trash` | `empty trash` | Empty the Trash | - |
| `Show Desktop` | `desktop` | Move windows aside | `F11` |
| `Quit All Apps` | `quit all` | Close all applications | - |
| `Hide All Apps` | `hide all` | Hide all except frontmost | `Cmd+Opt+H` |
| `Unhide All Apps` | `unhide all` | Unhide all hidden apps | - |
| `Toggle Hidden Files` | `show hidden` | Show/hide hidden files | - |
| `Toggle Appearance` | `dark mode`, `light mode` | Switch theme | - |
| `Eject All Disks` | `eject all` | Safely eject all disks | - |

### Volume Controls

| Command | Description |
|---------|-------------|
| `Volume 0%` / `mute` | Mute audio |
| `Volume 25%` | Set to 25% |
| `Volume 50%` | Set to 50% |
| `Volume 75%` | Set to 75% |
| `Volume 100%` / `max volume` | Maximum volume |
| `Volume Up` | Increase by 6.25% |
| `Volume Down` | Decrease by 6.25% |
| `Toggle Mute` | Toggle mute state |

### Media Controls

| Command | Description |
|---------|-------------|
| `Play/Pause` | Toggle media playback |
| `Next Track` | Skip to next track |
| `Previous Track` | Go to previous track |

### Bluetooth

| Command | Description |
|---------|-------------|
| `Toggle Bluetooth` | Turn Bluetooth on/off |
| `Bluetooth On` | Enable Bluetooth |
| `Bluetooth Off` | Disable Bluetooth |

### Implementation

```rust
use cocoa::appkit::NSWorkspace;
use core_foundation::runloop::CFRunLoopRunInMode;

pub enum SystemCommand {
    LockScreen,
    Sleep,
    SleepDisplays,
    Restart,
    ShutDown,
    LogOut,
    EmptyTrash,
    ShowDesktop,
    QuitAllApps,
    HideAllApps,
    UnhideAllApps,
    ToggleHiddenFiles,
    ToggleAppearance,
    EjectAllDisks,
    SetVolume(f32),
    ToggleMute,
    MediaPlayPause,
    MediaNext,
    MediaPrevious,
    ToggleBluetooth,
}

impl SystemCommand {
    pub fn execute(&self) -> Result<()> {
        match self {
            SystemCommand::LockScreen => {
                // Use CGSession to lock
                unsafe {
                    let workspace = NSWorkspace::sharedWorkspace(nil);
                    // SACLockScreenImmediate
                }
            }
            SystemCommand::Sleep => {
                Command::new("pmset")
                    .args(["sleepnow"])
                    .spawn()?;
            }
            SystemCommand::EmptyTrash => {
                // Use NSFileManager or Finder scripting
                let script = r#"
                    tell application "Finder"
                        empty trash
                    end tell
                "#;
                Command::new("osascript")
                    .args(["-e", script])
                    .spawn()?;
            }
            SystemCommand::SetVolume(level) => {
                let percent = (level * 100.0) as i32;
                Command::new("osascript")
                    .args(["-e", &format!("set volume output volume {}", percent)])
                    .spawn()?;
            }
            // ... other commands
        }
        Ok(())
    }
}
```

---

## Calendar Integration

View and manage calendar events directly from PhotonCast.

### Commands

| Command | Description |
|---------|-------------|
| `My Schedule` | View upcoming events |
| `Today's Events` | Events for today |
| `This Week` | Events for current week |
| `Create Event` | Quick event creation |
| `Join Meeting` | Join next conference call |

### Features

**Event Display:**
- Event title, time, and duration
- Location and conference links
- Attendees list
- Calendar color coding

**Conference Integration:**
- Auto-detect Zoom, Google Meet, Teams links
- One-click join meeting
- Show "Join" button when meeting is starting

**Quick Actions:**
- Join conference call
- Copy event details
- Open in Calendar app
- Email attendees
- Block focus time

### Implementation

```rust
use objc2_event_kit::{EKEventStore, EKAuthorizationStatus, EKEntityType};

pub struct CalendarProvider {
    store: EKEventStore,
    calendars: Vec<CalendarInfo>,
}

#[derive(Debug)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub attendees: Vec<Attendee>,
    pub conference_url: Option<String>,
    pub calendar: CalendarInfo,
    pub is_all_day: bool,
}

impl CalendarProvider {
    pub async fn request_access(&self) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        self.store.requestAccessToEntityType_completion(
            EKEntityType::Event,
            |granted, error| {
                tx.send(granted).ok();
            }
        );
        rx.await.map_err(|_| Error::CalendarAccessDenied)
    }
    
    pub fn get_events(&self, start: DateTime<Local>, end: DateTime<Local>) -> Result<Vec<CalendarEvent>> {
        let predicate = self.store.predicateForEventsWithStartDate_endDate_calendars(
            start.into(),
            end.into(),
            None,
        );
        
        let events = self.store.eventsMatchingPredicate(predicate);
        Ok(events.into_iter().map(CalendarEvent::from).collect())
    }
    
    pub fn detect_conference_url(&self, event: &CalendarEvent) -> Option<String> {
        // Check location field
        if let Some(loc) = &event.location {
            if let Some(url) = extract_meeting_url(loc) {
                return Some(url);
            }
        }
        
        // Check notes/description
        if let Some(notes) = &event.notes {
            if let Some(url) = extract_meeting_url(notes) {
                return Some(url);
            }
        }
        
        None
    }
}

fn extract_meeting_url(text: &str) -> Option<String> {
    // Zoom
    if let Some(cap) = ZOOM_REGEX.captures(text) {
        return Some(cap[0].to_string());
    }
    // Google Meet
    if let Some(cap) = MEET_REGEX.captures(text) {
        return Some(cap[0].to_string());
    }
    // Teams
    if let Some(cap) = TEAMS_REGEX.captures(text) {
        return Some(cap[0].to_string());
    }
    None
}
```

### Calendar Providers

Supported via macOS native calendar:
- iCloud Calendar
- Google Calendar
- Microsoft Exchange/Outlook
- Yahoo Calendar
- CalDAV providers

---

## Clipboard History

Track and recall clipboard content.

### Features

| Feature | Description |
|---------|-------------|
| **Text** | Plain text with preview |
| **Rich Text** | HTML/RTF formatting preserved |
| **Images** | Thumbnails with full view |
| **Files** | File references with icons |
| **Links** | URL detection and preview |
| **Colors** | Hex/RGB color detection |

### Commands

| Command | Description | Shortcut |
|---------|-------------|----------|
| `Clipboard History` | Browse history | `Cmd+Shift+V` |
| `Clear Clipboard` | Clear history | - |
| `Pin Item` | Pin to top | - |
| `Search Clipboard` | Search history | - |

### Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `historySize` | 1000 | Maximum items to store |
| `retentionDays` | 30 | Days to keep items |
| `excludeApps` | [] | Apps to ignore |
| `excludePasswords` | true | Ignore password managers |
| `storeImages` | true | Store image content |

### Implementation

```rust
use cocoa::appkit::NSPasteboard;
use rusqlite::{Connection, params};

pub struct ClipboardManager {
    db: Connection,
    last_change_count: i64,
    excluded_apps: HashSet<String>,
}

#[derive(Debug)]
pub struct ClipboardItem {
    pub id: i64,
    pub content_type: ContentType,
    pub text: Option<String>,
    pub html: Option<String>,
    pub image_path: Option<PathBuf>,
    pub file_paths: Vec<PathBuf>,
    pub source_app: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_pinned: bool,
}

#[derive(Debug)]
pub enum ContentType {
    Text,
    RichText,
    Image,
    File,
    Color,
}

impl ClipboardManager {
    pub fn start_monitoring(&mut self) {
        let pasteboard = unsafe { NSPasteboard::generalPasteboard(nil) };
        
        loop {
            let change_count = unsafe { pasteboard.changeCount() };
            
            if change_count != self.last_change_count {
                self.last_change_count = change_count;
                
                if let Ok(item) = self.capture_content(&pasteboard) {
                    // Check if from excluded app
                    if !self.should_exclude(&item) {
                        self.store_item(item);
                    }
                }
            }
            
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    
    pub fn search(&self, query: &str) -> Result<Vec<ClipboardItem>> {
        let items = self.db.prepare(
            "SELECT * FROM clipboard_history 
             WHERE text LIKE ?1 
             ORDER BY is_pinned DESC, created_at DESC 
             LIMIT 100"
        )?
        .query_map(params![format!("%{}%", query)], ClipboardItem::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(items)
    }
}
```

---

## Window Management

Position and resize windows with keyboard commands.

### Layouts

| Command | Position | Size |
|---------|----------|------|
| `Left Half` | Left edge | 50% width |
| `Right Half` | Right edge | 50% width |
| `Top Half` | Top edge | 50% height |
| `Bottom Half` | Bottom edge | 50% height |
| `Top Left` | Top-left corner | 25% |
| `Top Right` | Top-right corner | 25% |
| `Bottom Left` | Bottom-left corner | 25% |
| `Bottom Right` | Bottom-right corner | 25% |
| `Maximize` | Full screen | 100% |
| `Center` | Centered | Current size |
| `Restore` | Previous position | Previous size |
| `First Third` | Left | 33% width |
| `Center Third` | Center | 33% width |
| `Last Third` | Right | 33% width |
| `First Two Thirds` | Left | 66% width |
| `Last Two Thirds` | Right | 66% width |

### Multi-Monitor

| Command | Description |
|---------|-------------|
| `Move to Next Display` | Move window to next monitor |
| `Move to Previous Display` | Move window to previous monitor |
| `Move to Display 1/2/3` | Move to specific monitor |

### Implementation

```rust
use accessibility::{AXUIElement, AXValue};
use core_graphics::display::CGDisplay;

pub struct WindowManager {
    accessibility_enabled: bool,
}

pub enum WindowPosition {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Maximize,
    Center,
    FirstThird,
    CenterThird,
    LastThird,
    FirstTwoThirds,
    LastTwoThirds,
}

impl WindowManager {
    pub fn position_window(&self, position: WindowPosition) -> Result<()> {
        let focused = self.get_focused_window()?;
        let screen = self.get_current_screen(&focused)?;
        let frame = self.calculate_frame(position, &screen);
        
        self.set_window_frame(&focused, frame)?;
        Ok(())
    }
    
    fn calculate_frame(&self, position: WindowPosition, screen: &ScreenInfo) -> CGRect {
        let (x, y, w, h) = match position {
            WindowPosition::LeftHalf => (
                screen.x,
                screen.y,
                screen.width / 2.0,
                screen.height,
            ),
            WindowPosition::RightHalf => (
                screen.x + screen.width / 2.0,
                screen.y,
                screen.width / 2.0,
                screen.height,
            ),
            WindowPosition::Maximize => (
                screen.x,
                screen.y,
                screen.width,
                screen.height,
            ),
            // ... other positions
        };
        
        CGRect::new(x, y, w, h)
    }
    
    fn set_window_frame(&self, window: &AXUIElement, frame: CGRect) -> Result<()> {
        let position = AXValue::from_cgpoint(CGPoint::new(frame.x, frame.y))?;
        let size = AXValue::from_cgsize(CGSize::new(frame.width, frame.height))?;
        
        window.set_attribute("AXPosition", &position)?;
        window.set_attribute("AXSize", &size)?;
        
        Ok(())
    }
}
```

---

## Application Management

### Commands

| Command | Description |
|---------|-------------|
| `Uninstall <App>` | Move app to Trash with cleanup |
| `Quit <App>` | Force quit application |
| `Show <App> Info` | Display app details |
| `Open <App> Settings` | Open app preferences |

### Uninstall Features

When uninstalling an app, PhotonCast can optionally remove:

| Location | Content |
|----------|---------|
| `~/Library/Application Support/<App>` | App data |
| `~/Library/Preferences/<bundle-id>.plist` | Preferences |
| `~/Library/Caches/<bundle-id>` | Cache files |
| `~/Library/Logs/<App>` | Log files |
| `~/Library/Saved Application State/<bundle-id>.savedState` | State |
| `~/Library/Containers/<bundle-id>` | Sandbox data |

### Implementation

```rust
pub struct AppManager {
    apps: Vec<Application>,
}

pub struct UninstallResult {
    pub app_path: PathBuf,
    pub related_files: Vec<RelatedFile>,
    pub total_size: u64,
}

pub struct RelatedFile {
    pub path: PathBuf,
    pub size: u64,
    pub category: FileCategory,
}

pub enum FileCategory {
    Preferences,
    ApplicationSupport,
    Caches,
    Logs,
    SavedState,
    Containers,
}

impl AppManager {
    pub fn find_related_files(&self, app: &Application) -> Result<Vec<RelatedFile>> {
        let mut files = Vec::new();
        let bundle_id = app.bundle_id.as_ref().ok_or(Error::NoBundleId)?;
        
        // Application Support
        let app_support = dirs::data_local_dir()
            .unwrap()
            .join("Application Support")
            .join(&app.name);
        if app_support.exists() {
            files.push(RelatedFile {
                path: app_support,
                size: dir_size(&app_support)?,
                category: FileCategory::ApplicationSupport,
            });
        }
        
        // Preferences
        let prefs = dirs::home_dir()
            .unwrap()
            .join("Library/Preferences")
            .join(format!("{}.plist", bundle_id));
        if prefs.exists() {
            files.push(RelatedFile {
                path: prefs.clone(),
                size: prefs.metadata()?.len(),
                category: FileCategory::Preferences,
            });
        }
        
        // ... check other locations
        
        Ok(files)
    }
    
    pub async fn uninstall(&self, app: &Application, remove_related: bool) -> Result<UninstallResult> {
        let related = self.find_related_files(app)?;
        
        // Move app to Trash
        trash::delete(&app.path)?;
        
        // Optionally remove related files
        if remove_related {
            for file in &related {
                trash::delete(&file.path)?;
            }
        }
        
        Ok(UninstallResult {
            app_path: app.path.clone(),
            related_files: related,
            total_size: 0, // calculated
        })
    }
}
```

---

## Sleep Timer

Schedule system actions after a delay.

### Commands

| Command | Action |
|---------|--------|
| `Sleep in 30 minutes` | Sleep after 30m |
| `Shut down in 1 hour` | Shutdown after 1h |
| `Lock in 15 minutes` | Lock after 15m |
| `Cancel timer` | Cancel scheduled action |
| `Show timer` | Show remaining time |

### Supported Durations

- Minutes: `5 min`, `15 minutes`, `30m`
- Hours: `1 hour`, `2h`, `1.5 hours`
- Time: `at 10pm`, `at 22:00`

### Implementation

```rust
use tokio::time::{sleep, Duration};

pub struct SleepTimer {
    active_timer: Option<TimerHandle>,
}

pub struct TimerHandle {
    action: ScheduledAction,
    scheduled_at: Instant,
    execute_at: Instant,
    cancel_tx: oneshot::Sender<()>,
}

pub enum ScheduledAction {
    Sleep,
    ShutDown,
    Restart,
    Lock,
    LogOut,
    Custom(String), // Custom script
}

impl SleepTimer {
    pub fn schedule(&mut self, action: ScheduledAction, delay: Duration) -> Result<()> {
        // Cancel existing timer
        self.cancel();
        
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let execute_at = Instant::now() + delay;
        
        let handle = TimerHandle {
            action: action.clone(),
            scheduled_at: Instant::now(),
            execute_at,
            cancel_tx,
        };
        
        self.active_timer = Some(handle);
        
        // Spawn timer task
        tokio::spawn(async move {
            tokio::select! {
                _ = sleep(delay) => {
                    action.execute().await;
                }
                _ = cancel_rx => {
                    // Timer cancelled
                }
            }
        });
        
        Ok(())
    }
    
    pub fn cancel(&mut self) {
        if let Some(handle) = self.active_timer.take() {
            handle.cancel_tx.send(()).ok();
        }
    }
    
    pub fn remaining(&self) -> Option<Duration> {
        self.active_timer.as_ref().map(|h| {
            h.execute_at.saturating_duration_since(Instant::now())
        })
    }
}
```

---

## Quick Links / Bookmarks

User-defined URL shortcuts.

### Features

- Create custom keyword aliases
- Import from browsers (Safari, Chrome, Firefox)
- Folder organization
- Search within bookmarks
- Favicon display

### Configuration

```toml
# ~/.config/photoncast/quicklinks.toml

[[links]]
title = "GitHub"
url = "https://github.com"
keywords = ["gh", "git"]
icon = "github"

[[links]]
title = "Gmail"
url = "https://mail.google.com"
keywords = ["mail", "email"]

[[links]]
title = "Notion"
url = "https://notion.so"
keywords = ["notes"]

[[folders]]
name = "Work"
links = [
  { title = "Jira", url = "https://company.atlassian.net", keywords = ["tickets"] },
  { title = "Confluence", url = "https://company.atlassian.net/wiki" },
]
```

---

## File Search

Quick access to files via Spotlight.

### Commands

| Command | Description |
|---------|-------------|
| `<filename>` | Search by name |
| `kind:pdf <query>` | Filter by type |
| `modified:today` | Recent files |
| `Open Downloads` | Open folder |

### File Type Filters

| Filter | Types |
|--------|-------|
| `kind:document` | doc, docx, pdf, txt |
| `kind:image` | jpg, png, gif, svg |
| `kind:video` | mp4, mov, avi |
| `kind:audio` | mp3, wav, aac |
| `kind:code` | rs, ts, py, js |
| `kind:archive` | zip, tar, gz |

### Actions

| Action | Shortcut |
|--------|----------|
| Open | `Enter` |
| Open with... | `Cmd+O` |
| Reveal in Finder | `Cmd+Shift+O` |
| Copy path | `Cmd+Shift+C` |
| Move to Trash | `Cmd+Backspace` |
| Quick Look | `Space` |
| Get Info | `Cmd+I` |

---

## System Preferences Shortcuts

Quick access to macOS settings.

### Supported Panels

| Command | Opens |
|---------|-------|
| `Display Settings` | Displays panel |
| `Sound Settings` | Sound panel |
| `Network Settings` | Network panel |
| `Bluetooth Settings` | Bluetooth panel |
| `Notifications Settings` | Notifications panel |
| `Privacy Settings` | Privacy & Security |
| `Keyboard Settings` | Keyboard panel |
| `Trackpad Settings` | Trackpad panel |
| `Accessibility Settings` | Accessibility panel |
| `Desktop & Dock` | Desktop & Dock panel |
| `Battery Settings` | Battery panel |
| `Users Settings` | Users & Groups |
| `Date & Time` | Date & Time panel |
| `Software Update` | Software Update |

### Implementation

```rust
pub fn open_system_preferences(panel: &str) -> Result<()> {
    let panel_id = match panel.to_lowercase().as_str() {
        "display" | "displays" => "com.apple.preference.displays",
        "sound" => "com.apple.preference.sound",
        "network" => "com.apple.preference.network",
        "bluetooth" => "com.apple.preference.bluetooth",
        "notifications" => "com.apple.preference.notifications",
        "privacy" | "security" => "com.apple.preference.security",
        "keyboard" => "com.apple.preference.keyboard",
        "trackpad" => "com.apple.preference.trackpad",
        "accessibility" => "com.apple.preference.universalaccess",
        "battery" => "com.apple.preference.battery",
        _ => return Err(Error::UnknownPanel(panel.to_string())),
    };
    
    Command::new("open")
        .arg(format!("x-apple.systempreferences:{}", panel_id))
        .spawn()?;
    
    Ok(())
}
```

---

## Snippets (Text Expansion)

Quick text insertion with placeholders.

### Features

- Static text snippets
- Dynamic placeholders (date, time, clipboard)
- Cursor positioning
- Multi-line snippets

### Placeholders

| Placeholder | Expands To |
|-------------|-----------|
| `{date}` | Current date |
| `{time}` | Current time |
| `{datetime}` | Date and time |
| `{clipboard}` | Clipboard content |
| `{cursor}` | Cursor position after expansion |
| `{uuid}` | Random UUID |

### Configuration

```toml
[[snippets]]
keyword = "!email"
content = "Best regards,\nJohn Doe"

[[snippets]]
keyword = "!meeting"
content = """
Meeting Notes - {date}
======================
Attendees: {cursor}

Discussion:
- 

Action Items:
- 
"""

[[snippets]]
keyword = "!sig"
content = """
--
John Doe
Software Engineer
john@example.com
"""
```

---

## Emoji Picker

Search and insert emoji.

### Features

- Search by name and keywords
- Recently used section
- Skin tone variants
- Copy or insert directly
- Categories: Smileys, People, Animals, Food, Activities, Travel, Objects, Symbols, Flags

---

## Color Picker

Pick and convert colors.

### Features

- Screen color picker (eyedropper)
- Color format conversion
- Color palette storage
- Recent colors

### Supported Formats

| Format | Example |
|--------|---------|
| HEX | `#FF5733` |
| RGB | `rgb(255, 87, 51)` |
| RGBA | `rgba(255, 87, 51, 0.8)` |
| HSL | `hsl(11, 100%, 60%)` |
| HSV | `hsv(11, 80%, 100%)` |
| CMYK | `cmyk(0, 66, 80, 0)` |
| Swift UIColor | `UIColor(red: 1.0, ...)` |
| SwiftUI Color | `Color(red: 1.0, ...)` |
| NSColor | `NSColor(red: 1.0, ...)` |
| CSS Variable | `var(--color-primary)` |

---

## Priority & Timeline

| Feature | Phase | Priority |
|---------|-------|----------|
| App Launcher | 1 (MVP) | P0 |
| System Commands | 1 (MVP) | P0 |
| File Search | 1 (MVP) | P0 |
| Calculator | 2 (v1.0) | P1 |
| Clipboard History | 2 (v1.0) | P1 |
| Window Management | 2 (v1.0) | P1 |
| Calendar | 2 (v1.0) | P1 |
| Quick Links | 2 (v1.0) | P2 |
| App Uninstall | 2 (v1.0) | P2 |
| Sleep Timer | 2 (v1.0) | P2 |
| System Preferences | 2 (v1.0) | P2 |
| Snippets | 3+ | P3 |
| Emoji Picker | 3+ | P3 |
| Color Picker | 3+ | P3 |
