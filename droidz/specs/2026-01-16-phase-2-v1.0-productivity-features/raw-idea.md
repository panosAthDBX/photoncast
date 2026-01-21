# PhotonCast Phase 2: Version 1.0 - Productivity Features

## Overview

Phase 2 takes PhotonCast from MVP (Phase 1 complete) to feature parity with basic Raycast/Alfred use cases. This is a comprehensive 3-month phase covering Sprints 4-6 (Weeks 13-24).

**Timeline:** Months 4-6
**Goal:** Feature parity with basic Raycast/Alfred use cases
**Release:** v1.0.0

---

## Sprint 4: Productivity Features (Weeks 13-16)

### 4.1 Clipboard History

Monitor and recall clipboard content with full history support.

**Core Features:**
- Monitor pasteboard changes via NSPasteboard
- Store clipboard history in SQLite database
- Support text, images, and file references
- Configurable history limit (default: 1000 items)
- Quick paste via keyboard shortcut (Cmd+Shift+V)
- Search through clipboard history
- Pin important items to top
- Exclude apps (e.g., password managers)

**Content Types:**
- Plain text with preview
- Rich text (HTML/RTF formatting preserved)
- Images (thumbnails with full view)
- Files (file references with icons)
- Links (URL detection and preview)
- Colors (Hex/RGB color detection)

**Configuration:**
- historySize: 1000 (default)
- retentionDays: 30
- excludeApps: []
- excludePasswords: true
- storeImages: true

### 4.2 Built-in Calculator

Natural language calculator supporting math, conversions, dates, and timezones.

**Basic Math:**
- Arithmetic: +, -, *, /, ^, %
- Functions: sqrt, sin, cos, tan, log, ln, abs, floor, ceil, round
- Constants: pi, e
- Parentheses for grouping
- Percentage calculations (32% of 500)

**Currency Conversions:**
- Major fiat currencies (USD, EUR, GBP, JPY, CNY, CAD, AUD, CHF)
- Cryptocurrency (BTC, ETH, USDT, BNB, XRP, ADA, DOGE, SOL)
- 150+ fiat currencies supported
- Background rate updates every 6 hours
- Natural language: "100 usd in eur", "0.5 btc in usd"

**Unit Conversions:**
- Length: mm, cm, m, km, in, ft, yd, mi
- Weight: mg, g, kg, oz, lb, ton
- Volume: ml, l, tsp, tbsp, cup, pt, qt, gal
- Temperature: C, F, K
- Data: B, KB, MB, GB, TB, PB
- Speed: m/s, km/h, mph, knots

**Date/Time Calculations:**
- Relative dates: "monday in 3 weeks", "35 days ago"
- Days until/since: "days until dec 25"
- Timezone conversions: "5pm ldn in sf", "2pm est to pst"
- Current time queries: "time in dubai"

---

## Sprint 5: Window Management & Productivity (Weeks 17-20)

### 5.1 Window Commands

Position and resize windows with keyboard commands.

**Layouts:**
- Half splits: left half, right half, top half, bottom half
- Quarters: top-left, top-right, bottom-left, bottom-right
- Maximize, center, restore
- Thirds: first third, center third, last third
- Two-thirds: first two thirds, last two thirds

**Multi-Monitor:**
- Move to next/previous display
- Move to specific display (1, 2, 3)

**Implementation:**
- Use Accessibility APIs (AXUIElement)
- Smooth animation during resize
- Request accessibility permissions

### 5.2 Quick Links

User-defined URL shortcuts with keywords.

**Features:**
- Create custom keyword aliases
- Import from browsers (Safari, Chrome, Firefox)
- Folder organization
- Search within bookmarks
- Favicon display
- Open in default browser

**Configuration (TOML):**
```toml
[[links]]
title = "GitHub"
url = "https://github.com"
keywords = ["gh", "git"]
icon = "github"
```

### 5.3 Calendar Integration

Connect to macOS native calendar via EventKit.

**Features:**
- Display upcoming events with color coding
- Conference call detection (Zoom, Meet, Teams links)
- One-click join meeting button
- Commands: My Schedule, Today's Events, This Week

**Quick Actions:**
- Join conference call
- Copy event details
- Open in Calendar app
- Email attendees

**Supported Calendars (via macOS):**
- iCloud Calendar
- Google Calendar
- Microsoft Exchange/Outlook
- CalDAV providers

### 5.4 App Management

Manage installed applications.

**Uninstaller:**
- Move app to Trash with cleanup
- Scan ~/Library for related files:
  - Application Support
  - Preferences (.plist)
  - Caches
  - Logs
  - Saved Application State
  - Containers (sandbox data)
- Show space to be freed
- Optional: remove preferences, caches, logs

**Other Commands:**
- Force quit applications
- Show app info (version, size, location)
- Open app settings

### 5.5 Sleep Timer

Schedule delayed system actions.

**Supported Actions:**
- Sleep in X minutes/hours
- Shut down at specific time
- Lock after delay
- Restart after delay

**Features:**
- Show remaining time
- Cancel scheduled timer
- Natural language parsing: "sleep in 30 min", "shutdown at 10pm"

### 5.6 Preferences & Settings

System preferences UI for PhotonCast.

**Settings:**
- Customizable global hotkey
- Theme selection:
  - Catppuccin Latte (light)
  - Catppuccin Frappé (dark - low contrast)
  - Catppuccin Macchiato (dark - medium contrast)
  - Catppuccin Mocha (dark - high contrast) [current default]
  - System theme detection and auto-switching
- Customizable accent color (14 options)
- Startup behavior settings
- Search scope configuration
- Keyboard shortcut customization

---

## Sprint 6: Native Extension System (Weeks 21-24)

### 6.1 Native Extension Architecture

Define and implement the native Rust extension system.

**Manifest Format (TOML):**
```toml
[extension]
name = "my-extension"
title = "My Extension"
description = "Does something useful"
version = "1.0.0"
author = "username"

[[commands]]
name = "main"
title = "Main Command"
mode = "view"
```

**Extension API:**
- Search provider API
- UI component API (List, Detail, Form, Grid)
- Storage API (per-extension SQLite)
- Clipboard access
- Toast/notification API

**Features:**
- Rust extension loading and lifecycle
- Hot-reload support for development
- Resource limits and sandboxing
- Extension enable/disable

### 6.2 Custom Commands

User-defined command shortcuts for power users.

**Features:**
- Shell script execution
- Environment variable support
- Output capture and display
- Error handling and notifications
- Keyboard shortcut binding

### 6.3 First-Party Native Extensions

Build reference implementations to demonstrate the extension API.

**Planned Extensions:**
1. **GitHub Repositories Browser** - Browse and search your repos
2. **System Preferences Shortcuts** - Quick access to all macOS settings
3. **Color Picker** - Screen eyedropper, format conversion, palette storage

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Daily active users | 500+ |
| GitHub stars | 1,000+ |
| Native extensions | 5+ |
| User-reported bugs | < 20 open |
| NPS score | 50+ |

## Acceptance Criteria

- Clipboard captures all copy events
- Calculator evaluates in under 5ms
- History is searchable
- Currency rates updated every 6 hours
- Windows resize smoothly
- Multi-monitor detection works
- Quick links open instantly
- Calendar events load in under 500ms
- App uninstall cleans up 90%+ of related files
- Native extensions load in under 50ms
- API is documented with examples
- Custom commands execute reliably

---

## Technical Considerations

**Dependencies to evaluate:**
- `evalexpr` or `meval` for math expression parsing
- `chrono` + `chrono-tz` for date/time handling
- Currency API (exchangerate-api.com or similar)
- `accessibility` crate for window management
- `objc2-event-kit` for calendar integration

**Permissions Required:**
- Accessibility (window management)
- Calendar (EventKit read/write)
- Automation (app control)

**Storage:**
- SQLite for clipboard history
- SQLite for extension storage
- TOML for quick links config
- JSON for preferences
