# Window Management - Raycast Parity Specification

> **Version:** 1.0.0  
> **Date:** 2026-01-21  
> **Status:** Draft  
> **Priority:** Phase 3

---

## Table of Contents

1. [Overview](#1-overview)
2. [User Stories](#2-user-stories)
3. [Feature Specifications](#3-feature-specifications)
4. [UI/UX Specifications](#4-uiux-specifications)
5. [Technical Implementation](#5-technical-implementation)
6. [Testing Requirements](#6-testing-requirements)
7. [Appendix: Raycast Feature Comparison](#appendix-raycast-feature-comparison)

---

## 1. Overview

### 1.1 Goal

Implement window management features with **100% Raycast parity**, allowing users to efficiently position, resize, and organize application windows using keyboard shortcuts directly from the launcher.

### 1.2 Raycast Feature Mapping

| Raycast Feature | PhotonCast Status | Priority |
|-----------------|-------------------|----------|
| **Halves** | | |
| Left Half | 🆕 New | P0 |
| Right Half | 🆕 New | P0 |
| Top Half | 🆕 New | P0 |
| Bottom Half | 🆕 New | P0 |
| **Quarters** | | |
| Top Left Quarter | 🆕 New | P0 |
| Top Right Quarter | 🆕 New | P0 |
| Bottom Left Quarter | 🆕 New | P0 |
| Bottom Right Quarter | 🆕 New | P0 |
| **Thirds** | | |
| First Third | 🆕 New | P1 |
| Center Third | 🆕 New | P1 |
| Last Third | 🆕 New | P1 |
| First Two Thirds | 🆕 New | P1 |
| Last Two Thirds | 🆕 New | P1 |
| **Centering** | | |
| Center | 🆕 New | P0 |
| Center Half | 🆕 New | P1 |
| Center Two Thirds | 🆕 New | P1 |
| **Maximize** | | |
| Maximize | 🆕 New | P0 |
| Almost Maximize | 🆕 New | P1 |
| Toggle Fullscreen | 🆕 New | P0 |
| **Restore** | | |
| Restore | 🆕 New | P0 |
| Reasonable Size | 🆕 New | P2 |
| **Cycling** | | |
| Cycle Sizes | 🆕 New | P1 |
| **Multi-Display** | | |
| Move to Next Display | 🆕 New | P0 |
| Move to Previous Display | 🆕 New | P0 |
| **Advanced** | | |
| Custom Hotkeys | 🆕 New | P1 |
| Window Gaps | 🆕 New | P2 |
| Make Smaller | 🆕 New | P2 |
| Make Larger | 🆕 New | P2 |

---

## 2. User Stories

### 2.1 Basic Window Positioning

**US-1:** As a user, I want to snap the focused window to the left/right half of my screen using a keyboard shortcut so I can work with two apps side-by-side.

**US-2:** As a user, I want to snap the focused window to any corner (quarter) of my screen so I can arrange four windows efficiently.

**US-3:** As a user, I want to maximize the focused window to fill the entire screen without entering macOS fullscreen mode.

### 2.2 Advanced Positioning

**US-4:** As a user, I want to position windows in thirds (1/3 or 2/3 of screen width) for better use of ultrawide monitors.

**US-5:** As a user, I want to center a window on my screen at its current size or at a specific size (half, two-thirds).

**US-6:** As a user, I want to cycle through different sizes when pressing the same shortcut repeatedly (e.g., left half → left third → left two-thirds).

### 2.3 Multi-Display

**US-7:** As a user, I want to move the focused window to my next/previous display while maintaining its relative position.

**US-8:** As a user with multiple monitors, I want windows to move seamlessly between displays.

### 2.4 Restore & History

**US-9:** As a user, I want to restore a window to its previous size and position after snapping it.

**US-10:** As a user, I want a "Reasonable Size" option that intelligently sizes windows based on their content.

### 2.5 Customization

**US-11:** As a user, I want to assign custom global hotkeys to window management commands.

**US-12:** As a user, I want to configure a gap between windows and screen edges.

---

## 3. Feature Specifications

### 3.1 Window Halves

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| Left Half | `⌃⌥←` | Move window to left 50% of screen |
| Right Half | `⌃⌥→` | Move window to right 50% of screen |
| Top Half | `⌃⌥↑` | Move window to top 50% of screen |
| Bottom Half | `⌃⌥↓` | Move window to bottom 50% of screen |

**Behavior:**
```
┌─────────────────┬─────────────────┐
│                 │                 │
│   Left Half     │   Right Half    │
│     (50%)       │     (50%)       │
│                 │                 │
└─────────────────┴─────────────────┘

┌─────────────────────────────────────┐
│            Top Half (50%)           │
├─────────────────────────────────────┤
│          Bottom Half (50%)          │
└─────────────────────────────────────┘
```

### 3.2 Window Quarters

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| Top Left Quarter | `⌃⌥U` | Move window to top-left 25% |
| Top Right Quarter | `⌃⌥I` | Move window to top-right 25% |
| Bottom Left Quarter | `⌃⌥J` | Move window to bottom-left 25% |
| Bottom Right Quarter | `⌃⌥K` | Move window to bottom-right 25% |

**Behavior:**
```
┌─────────────────┬─────────────────┐
│   Top Left      │   Top Right     │
│    Quarter      │    Quarter      │
├─────────────────┼─────────────────┤
│  Bottom Left    │  Bottom Right   │
│    Quarter      │    Quarter      │
└─────────────────┴─────────────────┘
```

### 3.3 Window Thirds

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| First Third | `⌃⌥D` | Left 33% of screen |
| Center Third | `⌃⌥F` | Center 33% of screen |
| Last Third | `⌃⌥G` | Right 33% of screen |
| First Two Thirds | `⌃⌥E` | Left 66% of screen |
| Last Two Thirds | `⌃⌥T` | Right 66% of screen |

**Behavior:**
```
┌───────────┬───────────┬───────────┐
│   First   │  Center   │   Last    │
│   Third   │   Third   │   Third   │
│   (33%)   │   (33%)   │   (33%)   │
└───────────┴───────────┴───────────┘

┌───────────────────────┬───────────┐
│   First Two Thirds    │   Last    │
│        (66%)          │   Third   │
└───────────────────────┴───────────┘
```

### 3.4 Centering

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| Center | `⌃⌥C` | Center window at current size |
| Center Half | `⌃⌥⇧C` | Center window at 50% screen width |
| Center Two Thirds | `⌃⌥⇧V` | Center window at 66% screen width |

**Behavior:**
```
┌─────────────────────────────────────┐
│                                     │
│    ┌─────────────────────────┐     │
│    │                         │     │
│    │   Centered Window       │     │
│    │                         │     │
│    └─────────────────────────┘     │
│                                     │
└─────────────────────────────────────┘
```

### 3.5 Maximize & Fullscreen

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| Maximize | `⌃⌥↵` | Fill screen (not macOS fullscreen) |
| Almost Maximize | `⌃⌥⇧↵` | Fill screen with small margin |
| Toggle Fullscreen | `⌃⌥⇧F` | Enter/exit macOS native fullscreen |

**Maximize vs Fullscreen:**
- **Maximize**: Window fills entire screen, menu bar and dock remain visible
- **Fullscreen**: Native macOS fullscreen mode (separate Space, hidden menu/dock)

**Almost Maximize:**
- Leaves 20px margin on all sides (configurable)
- Useful for quick access to desktop icons

### 3.6 Restore

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| Restore | `⌃⌥⌫` | Return to previous size/position |
| Reasonable Size | `⌃⌥⇧R` | Intelligently size window |

**Restore Behavior:**
- Stores previous window frame before any snap operation
- Pressing Restore returns window to exact previous size and position
- If no previous state, centers window at "reasonable" size

**Reasonable Size Algorithm:**
```rust
fn reasonable_size(window: &Window, screen: &Screen) -> CGRect {
    // Target: 75% of screen width, 80% of screen height
    // Minimum: 800x600
    // Maximum: screen size minus margins
    let target_width = (screen.width * 0.75).max(800.0).min(screen.width - 100.0);
    let target_height = (screen.height * 0.80).max(600.0).min(screen.height - 100.0);
    
    // Center the window
    let x = (screen.width - target_width) / 2.0;
    let y = (screen.height - target_height) / 2.0;
    
    CGRect::new(x, y, target_width, target_height)
}
```

### 3.7 Cycle Sizes

**Behavior:**
When pressing the same positioning shortcut multiple times, cycle through related sizes:

| Initial Position | Cycle Order |
|------------------|-------------|
| Left Half | Half → Third → Two Thirds → Half |
| Right Half | Half → Third → Two Thirds → Half |
| Top Half | Half → Third → Two Thirds → Half |
| Bottom Half | Half → Third → Two Thirds → Half |
| Center | Current Size → Half → Two Thirds → Current Size |

**Implementation:**
```rust
pub struct WindowManager {
    /// Tracks last command for cycling
    last_command: Option<(WindowCommand, Instant)>,
    /// Cycle timeout (ms)
    cycle_timeout: u64,
}

impl WindowManager {
    fn should_cycle(&self, command: WindowCommand) -> bool {
        if let Some((last_cmd, timestamp)) = &self.last_command {
            *last_cmd == command && timestamp.elapsed() < Duration::from_millis(self.cycle_timeout)
        } else {
            false
        }
    }
}
```

### 3.8 Multi-Display Support

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| Move to Next Display | `⌃⌥⇧→` | Move window to next display |
| Move to Previous Display | `⌃⌥⇧←` | Move window to previous display |

**Behavior:**
- Window maintains its relative position on the new display
- If window was in "left half" on Display 1, it stays in "left half" on Display 2
- Display order follows System Preferences → Displays arrangement
- Wraps around (after last display, goes to first)

**Position Preservation:**
```rust
fn move_to_display(window: &Window, target_display: &Display) -> CGRect {
    let current_display = window.current_display();
    
    // Calculate relative position (0.0 - 1.0)
    let rel_x = (window.frame.x - current_display.bounds.x) / current_display.bounds.width;
    let rel_y = (window.frame.y - current_display.bounds.y) / current_display.bounds.height;
    let rel_width = window.frame.width / current_display.bounds.width;
    let rel_height = window.frame.height / current_display.bounds.height;
    
    // Apply to new display
    CGRect::new(
        target_display.bounds.x + rel_x * target_display.bounds.width,
        target_display.bounds.y + rel_y * target_display.bounds.height,
        rel_width * target_display.bounds.width,
        rel_height * target_display.bounds.height,
    )
}
```

### 3.9 Window Gaps

**Configuration:**
- Default gap: 0px (no gap)
- Configurable: 0-50px
- Applied to all edges (screen edges and between windows)

**Settings Location:** Preferences → Extensions → Window Management → Gap

```rust
pub struct WindowManagementConfig {
    /// Gap between windows and screen edges in pixels
    pub window_gap: u32,
    /// Whether to include menu bar area
    pub respect_menu_bar: bool,
    /// Whether to include dock area
    pub respect_dock: bool,
}
```

### 3.10 Resize Commands

**Commands:**

| Command | Default Shortcut | Description |
|---------|------------------|-------------|
| Make Smaller | `⌃⌥-` | Shrink window by 10% |
| Make Larger | `⌃⌥=` | Grow window by 10% |

**Behavior:**
- Resizes from center (window stays centered)
- Minimum size: 400x300
- Maximum size: screen size

---

## 4. UI/UX Specifications

### 4.1 Command Discovery

Window management commands appear in search:
```
┌─────────────────────────────────────────────────────────────┐
│ 🔍 left half                                                │
├─────────────────────────────────────────────────────────────┤
│ [⊞] Left Half                                        ⌃⌥←  │
│     Move window to left half of screen                     │
├─────────────────────────────────────────────────────────────┤
│ [⊞] First Two Thirds                                 ⌃⌥E  │
│     Move window to left two thirds of screen               │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Command Icons

| Command Type | Icon |
|--------------|------|
| Halves | ◧ ◨ ⬒ ⬓ |
| Quarters | ◰ ◳ ◱ ◲ |
| Thirds | ▤ |
| Center | ◎ |
| Maximize | ⤢ |
| Restore | ⤡ |
| Display Move | ⇨ ⇦ |

### 4.3 Visual Feedback (Optional Enhancement)

When a window management command is executed, briefly show an overlay indicating the target position:

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   ┌────────────────────┐                                   │
│   │                    │                                   │
│   │   [HIGHLIGHT]      │         (dimmed area)             │
│   │   Left Half        │                                   │
│   │                    │                                   │
│   └────────────────────┘                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

- Overlay appears for 200ms
- Blue highlight on target area
- Dimmed overlay on rest of screen
- Can be disabled in preferences

---

## 5. Technical Implementation

### 5.1 New Crate: `photoncast-window`

```
crates/photoncast-window/
├── src/
│   ├── lib.rs           # Public API
│   ├── manager.rs       # WindowManager implementation
│   ├── commands.rs      # All window commands
│   ├── accessibility.rs # macOS Accessibility API wrapper
│   ├── display.rs       # Multi-display support
│   ├── animation.rs     # Window animation (optional)
│   └── config.rs        # User preferences
```

### 5.2 macOS APIs Required

| Feature | API |
|---------|-----|
| Get focused window | `AXUIElementCopyAttributeValue(kAXFocusedWindowAttribute)` |
| Get window frame | `AXUIElementCopyAttributeValue(kAXPositionAttribute/kAXSizeAttribute)` |
| Set window frame | `AXUIElementSetAttributeValue(kAXPositionAttribute/kAXSizeAttribute)` |
| Get displays | `CGGetActiveDisplayList` / `NSScreen.screens` |
| Get display bounds | `CGDisplayBounds` / `NSScreen.frame` |
| Get visible area | `NSScreen.visibleFrame` (excludes menu bar and dock) |
| Toggle fullscreen | `AXUIElementSetAttributeValue(kAXFullscreenAttribute)` |
| Accessibility check | `AXIsProcessTrusted` |

### 5.3 Accessibility Permissions

**Required Permission:** Accessibility

**First Run Flow:**
1. Check if accessibility is granted: `AXIsProcessTrusted()`
2. If not, show permission request dialog
3. Open System Preferences → Privacy → Accessibility
4. User grants permission
5. PhotonCast can now control windows

```rust
pub fn check_accessibility_permission() -> bool {
    unsafe {
        AXIsProcessTrusted()
    }
}

pub fn request_accessibility_permission() {
    let options = CFDictionaryCreate(
        kCFAllocatorDefault,
        &[kAXTrustedCheckOptionPrompt as *const _],
        &[kCFBooleanTrue as *const _],
        1,
        &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks,
    );
    AXIsProcessTrustedWithOptions(options);
}
```

### 5.4 Window Frame Calculation

```rust
pub struct ScreenBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ScreenBounds {
    /// Get visible bounds (excluding menu bar and dock)
    pub fn visible_bounds(&self, gap: u32) -> Self {
        Self {
            x: self.x + gap as f64,
            y: self.y + gap as f64,
            width: self.width - (gap * 2) as f64,
            height: self.height - (gap * 2) as f64,
        }
    }
    
    /// Calculate frame for left half
    pub fn left_half(&self) -> CGRect {
        CGRect::new(self.x, self.y, self.width / 2.0, self.height)
    }
    
    /// Calculate frame for right half
    pub fn right_half(&self) -> CGRect {
        CGRect::new(self.x + self.width / 2.0, self.y, self.width / 2.0, self.height)
    }
    
    /// Calculate frame for top-left quarter
    pub fn top_left_quarter(&self) -> CGRect {
        CGRect::new(self.x, self.y, self.width / 2.0, self.height / 2.0)
    }
    
    // ... etc for all positions
}
```

### 5.5 Window State Storage

Store previous window states for Restore functionality:

```rust
pub struct WindowHistory {
    /// Window identifier (process ID + window ID)
    frames: HashMap<WindowId, Vec<SavedFrame>>,
}

pub struct SavedFrame {
    pub frame: CGRect,
    pub timestamp: Instant,
}

impl WindowHistory {
    const MAX_HISTORY: usize = 10;
    
    pub fn save(&mut self, window_id: WindowId, frame: CGRect) {
        let history = self.frames.entry(window_id).or_default();
        history.push(SavedFrame {
            frame,
            timestamp: Instant::now(),
        });
        if history.len() > Self::MAX_HISTORY {
            history.remove(0);
        }
    }
    
    pub fn restore(&mut self, window_id: WindowId) -> Option<CGRect> {
        self.frames.get_mut(&window_id).and_then(|h| h.pop()).map(|s| s.frame)
    }
}
```

### 5.6 Global Hotkey Registration

Use the existing hotkey system from the main app:

```rust
// In main.rs or preferences
let window_shortcuts = vec![
    ("Left Half", "ctrl-option-left"),
    ("Right Half", "ctrl-option-right"),
    ("Maximize", "ctrl-option-return"),
    // ... etc
];

for (name, shortcut) in window_shortcuts {
    register_global_hotkey(shortcut, move |cx| {
        WindowManager::execute_command(name, cx);
    });
}
```

---

## 6. Testing Requirements

### 6.1 Unit Tests

| Test | Description |
|------|-------------|
| `test_frame_calculation_halves` | Verify half positions are calculated correctly |
| `test_frame_calculation_quarters` | Verify quarter positions |
| `test_frame_calculation_thirds` | Verify third positions |
| `test_frame_calculation_with_gap` | Verify gaps are applied correctly |
| `test_cycle_detection` | Verify cycling logic |
| `test_restore_history` | Verify window history is saved/restored |
| `test_multi_display_position` | Verify relative position is preserved |

### 6.2 Integration Tests

| Test | Description |
|------|-------------|
| `test_accessibility_permission_check` | Verify permission check works |
| `test_window_move_left_half` | Actually move a test window |
| `test_window_maximize` | Verify maximize fills screen |
| `test_window_restore` | Verify restore returns to previous position |
| `test_multi_display_move` | Move window between displays |

### 6.3 Manual Testing Checklist

- [ ] Left/Right half snapping
- [ ] Top/Bottom half snapping
- [ ] All four quarter positions
- [ ] All five third positions
- [ ] Center at current size
- [ ] Center at half/two-thirds
- [ ] Maximize (not fullscreen)
- [ ] Almost maximize
- [ ] Toggle fullscreen
- [ ] Restore previous position
- [ ] Cycle through sizes
- [ ] Move to next/previous display
- [ ] Window gaps applied correctly
- [ ] Shortcuts work from PhotonCast search
- [ ] Shortcuts work as global hotkeys
- [ ] Works with different screen resolutions
- [ ] Works with external displays
- [ ] Works with menu bar on different sides
- [ ] Works with dock on different sides

---

## Appendix: Raycast Feature Comparison

### A.1 Feature Parity Matrix

| Feature | Raycast | PhotonCast (After) | Notes |
|---------|---------|-------------------|-------|
| **Halves** | | | |
| Left Half | ✅ | ✅ | |
| Right Half | ✅ | ✅ | |
| Top Half | ✅ | ✅ | |
| Bottom Half | ✅ | ✅ | |
| **Quarters** | | | |
| Top Left | ✅ | ✅ | |
| Top Right | ✅ | ✅ | |
| Bottom Left | ✅ | ✅ | |
| Bottom Right | ✅ | ✅ | |
| **Thirds** | | | |
| First Third | ✅ | ✅ | |
| Center Third | ✅ | ✅ | |
| Last Third | ✅ | ✅ | |
| First Two Thirds | ✅ | ✅ | |
| Last Two Thirds | ✅ | ✅ | |
| **Sixths** | ✅ | ❌ | P3 - Low priority |
| **Centering** | | | |
| Center | ✅ | ✅ | |
| Center Half | ✅ | ✅ | |
| Center Two Thirds | ✅ | ✅ | |
| **Maximize** | | | |
| Maximize | ✅ | ✅ | |
| Almost Maximize | ✅ | ✅ | |
| Toggle Fullscreen | ✅ | ✅ | |
| **Restore** | | | |
| Restore | ✅ | ✅ | |
| Reasonable Size | ✅ | ✅ | |
| **Cycling** | | | |
| Cycle Sizes | ✅ | ✅ | |
| **Multi-Display** | | | |
| Next Display | ✅ | ✅ | |
| Previous Display | ✅ | ✅ | |
| **Customization** | | | |
| Custom Hotkeys | ✅ | ✅ | |
| Window Gaps | ✅ Pro | ✅ | |
| **Resize** | | | |
| Make Smaller | ✅ | ✅ | |
| Make Larger | ✅ | ✅ | |

### A.2 Shortcuts Comparison

| Command | Raycast Default | PhotonCast Default |
|---------|-----------------|-------------------|
| Left Half | (User configurable) | `⌃⌥←` |
| Right Half | (User configurable) | `⌃⌥→` |
| Top Half | (User configurable) | `⌃⌥↑` |
| Bottom Half | (User configurable) | `⌃⌥↓` |
| Maximize | (User configurable) | `⌃⌥↵` |
| Restore | (User configurable) | `⌃⌥⌫` |
| Toggle Fullscreen | (User configurable) | `⌃⌥⇧F` |
| Next Display | (User configurable) | `⌃⌥⇧→` |
| Previous Display | (User configurable) | `⌃⌥⇧←` |

### A.3 Not Implemented (Intentional)

| Raycast Feature | Reason Not Implemented |
|-----------------|------------------------|
| Sixths | Low demand, complex UI |
| Window Layouts (save/restore) | Pro feature in Raycast |
| Per-app window rules | Pro feature in Raycast |

---

*End of Specification*
