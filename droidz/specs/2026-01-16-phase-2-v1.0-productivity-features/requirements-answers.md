# Phase 2: Requirements Answers

> Answered: 2026-01-16

---

## Sprint 4: Productivity Features

### 4.1 Clipboard History

| # | Question | Answer |
|---|----------|--------|
| 1 | Default hotkey | **A) `Cmd+Shift+V`** |
| 2 | Retention period | **B) 30 days** |
| 3 | Max items stored | **D) Configurable with default 1000** |
| 4 | Persist across restarts | **B) Yes (encrypted SQLite)** |
| 5 | Encryption | **B) AES-256 with machine-derived key** |
| 6 | Support images | **A) Yes (store full images)** |
| 7 | Max image size | **B) 10MB** |
| 8 | File references | **A) Yes (store path + icon + metadata)** |
| 9 | Color detection | **A) Yes (with color swatch preview)** |
| 10 | Rich text handling | **A) Preserve formatting with paste options** |
| 11 | URL detection | **A) Yes (fetch favicon + title in background)** |
| 12 | Excluded apps | **Use suggested defaults** (1Password, Bitwarden, LastPass, Keychain, Dashlane) + user configurable |
| 13 | Transient items | **A) Yes (never store transient items)** |
| 14 | Concealed mode | **A) Yes ("Paste and Don't Save" action)** |
| 15 | Clear history | **A) Yes (with confirmation dialog)** |
| 16 | Pin items | **B) Yes (separate "Pinned" section)** |
| 17 | Pinned vs limit | **B) No (pinned items separate from limit)** |
| 18 | Merge items | **C) Phase 3+** |
| 19 | OCR for images | **C) Phase 3+** |
| 20 | Enter action | **C) Configurable with default "Paste directly"** |
| 21 | Sync across devices | **A) No (local only)** |

### 4.2 Calculator

**All suggestions accepted (questions 22-40)**

| # | Question | Answer |
|---|----------|--------|
| 22 | Decimal precision | A) f64 for most, B) Decimal128 for currency |
| 23 | Expression parser | A) evalexpr |
| 24 | Math functions | Include all suggested (basic + inverse trig + hyperbolic + exp, pow, mod, min, max, factorial) |
| 25 | Variable assignment | B) No (v1.0) |
| 26 | Currency API | C) frankfurter.app |
| 27 | Rate update frequency | B) Every 6 hours |
| 28 | Offline cache | A) SQLite cache with timestamp |
| 29 | Cryptocurrencies | Add USDC, MATIC, AVAX, DOT, LINK to make top ~15 |
| 30 | Crypto API | A) CoinGecko |
| 31 | Case sensitivity | A) Case-insensitive |
| 32 | Unit abbreviations | A) Comprehensive aliases |
| 33 | Speed units | Add ft/s, defer Mach |
| 34 | Date parsing | B) dateparser or A) chrono-english (evaluate both) |
| 35 | Timezone resolution | A) Common US/EU conventions with system locale fallback |
| 36 | City-based timezones | A) Yes |
| 37 | City database | A) Bundled minimal (~500 cities) |
| 38 | Real-time results | A) Real-time with debounce |
| 39 | Copy currency result | C) Both actions available |
| 40 | Calculator history | A) Yes (separate command) |

---

## Sprint 5: Window Management & Productivity

### 5.1 Window Management

**All suggestions accepted (questions 41-52)**

| # | Question | Answer |
|---|----------|--------|
| 41 | Dedicated hotkeys | C) Both with suggestions |
| 42 | Default hotkey scheme | C) No defaults, provide suggestions in preferences |
| 43 | Cycling behavior | A) Yes (cycling between sizes) |
| 44 | Custom grid layouts | B) No (v1.0), preset layouts only |
| 45 | Snap to edges | B) No (command-only) |
| 46 | Animation | B) Yes (respecting Reduce Motion) |
| 47 | Animation duration | B) 200ms |
| 48 | Animation configurable | A) Yes (in preferences) |
| 49 | Multi-monitor cycling | A) Cycle by macOS arrangement |
| 50 | Preserve position on move | A) Preserve relative position |
| 51 | Display disconnect handling | B) No (let macOS handle) |
| 52 | Accessibility permissions | C) Both (prompt + inline status) |

### 5.2 Quick Links

**Suggestions accepted with addition:**

| # | Question | Answer |
|---|----------|--------|
| 53 | Storage format | B) SQLite + TOML export |
| 54 | Dynamic URL parameters | A) Yes (with input prompt) |
| 55 | Browser import | **A) Yes - Safari, Chrome, Firefox + Arc (added)** |
| 56 | Organization | B) Yes (tags) |

### 5.3 Calendar Integration

**Suggestions accepted with clarification:**

| # | Question | Answer |
|---|----------|--------|
| 57 | Permission request | A) On first calendar command |
| 58 | Access level | A) Read-only for v1.0 |
| 59 | Days shown | B) 7 days |
| 60 | All-day events | A) Grouped at top |
| 61 | Recurring events | A) Individual instances |
| 62 | Timezone handling | A) Convert to local with indicator |
| 63 | Conference providers | A) Zoom + Meet + Teams |
| 64 | Conference link detection | C) All fields + structured data |
| 65 | Join button timing | B) Within 15 minutes |
| 66 | Focus mode integration | C) No (v1.0) |

**Additional requirement:** Selecting a meeting and pressing **Enter should join the meeting** (like Raycast behavior). This should be the primary action for events with conference links.

### 5.4 App Management

**Suggestions modified:**

| # | Question | Answer |
|---|----------|--------|
| 67 | Uninstall confirmation | B) Yes (with preview) |
| 68 | System app protection | A) Yes (hardcoded) |
| 69 | Show space freed | A) Yes |
| 70 | Related file cleanup | A) Conservative for safety |
| 71 | Deep scan approach | **C) Optional deep scan, but DEFAULT ON** (changed from default off) |
| 72 | Trash vs permanent | A) Move to Trash |
| 73 | Force quit confirmation | B) Yes (for non-frozen apps) |
| 74 | Not Responding indicator | A) Yes |

**Additional requirements:**
- **App Sleep feature**: Like Raycast, put apps to sleep (stop them) after X minutes of idle time. This should be configurable per-app.
- **Quit vs Force Quit**: Both are super important, make sure both are prominent actions
- Replicate full Raycast app management feature set

### 5.5 Sleep Timer

**All suggestions accepted:**

| # | Question | Answer |
|---|----------|--------|
| 75 | Persist across restarts | A) Yes (persist to disk) |
| 76 | Confirmation before action | A) Yes (1 minute warning with cancel) |
| 77 | Multiple timers | B) No (one timer at a time) |
| 78 | Countdown indicator | C) Both (menu bar + launcher) |
| 79 | Cancel on user activity | B) No (strict timer) |

### 5.6 Preferences & Settings

**All suggestions accepted:**

| # | Question | Answer |
|---|----------|--------|
| 80 | Storage location | B) ~/.config/photoncast/config.toml |
| 81 | Import/export | B) TOML file is directly shareable |
| 82 | Catppuccin variants | A) Yes (all four: Latte, Frappé, Macchiato, Mocha) |
| 83 | Follow system appearance | A) Yes (auto option) |
| 84 | Accent colors | 14 Catppuccin named colors (Rosewater, Flamingo, Pink, Mauve, Red, Maroon, Peach, Yellow, Green, Teal, Sky, Sapphire, Blue, Lavender) |
| 85 | Customize shortcuts | A) Yes (fully customizable) |
| 86 | Hyper key support | A) Yes |

---

## Sprint 6: Native Extension System

### 6.1 Extension Architecture

**All suggestions accepted:**

| # | Question | Answer |
|---|----------|--------|
| 87 | Manifest format | A) TOML |
| 88 | Installation location | B) ~/Library/Application Support/PhotonCast/Extensions/ |
| 89 | Sandboxing | B) Yes (permissioned sandbox) |
| 90 | Permissions model | clipboard_read, clipboard_write, network, filesystem_read, filesystem_write, notifications, storage |
| 91 | API sync/async | A) Async-first |
| 92 | UI components | List, Detail, Form, Grid, Action, ActionPanel |
| 93 | Search integration | A) Yes (extensions can provide search results) |
| 94 | Error handling | C) Both (isolated process + error boundaries) |
| 95 | Hot reload | A) Yes (file watcher in dev mode) |
| 96 | Development CLI | A) Yes (full CLI with scaffolding, validation, packaging) |

### 6.2 Custom Commands

**All suggestions accepted:**

| # | Question | Answer |
|---|----------|--------|
| 97 | Shell | C) Configurable per command, default to $SHELL |
| 98 | Output streaming | A) Streamed (real-time) |
| 99 | Timeout | D) Configurable per command, 60 second default |
| 100 | Environment variables | C) Both (inherit system + per-command) |
| 101 | Interactive commands | B) No (v1.0) |
| 102 | Completion notification | C) HUD for success, Toast for failure |

### 6.3 First-Party Extensions

**All suggestions accepted:**

| # | Question | Answer |
|---|----------|--------|
| 103 | Which extensions | GitHub Repos Browser, System Preferences Shortcuts, Color Picker |
| 104 | Bundled vs downloaded | A) Bundled |

---

## Cross-Cutting Concerns

### Testing & Quality

| # | Question | Answer |
|---|----------|--------|
| 105 | Test coverage target | C) 80% |
| 106 | Integration tests | A) Yes (all features) |
| 107 | UI tests | C) Critical components only |

### Performance

| # | Question | Answer |
|---|----------|--------|
| 108 | Performance targets | Calculator <5ms, Clipboard load <100ms, Window resize <50ms, Calendar <500ms, Extension load <50ms |
| 109 | Benchmarks | B) Yes (performance-critical only) |

### Documentation

| # | Question | Answer |
|---|----------|--------|
| 110 | Documentation scope | B) Full user guide + API reference |
| 111 | Extension tutorials | A) Yes (v1.0) |
| 112 | API stability | C) Warn about unstable APIs (mark as `#[unstable]`) |

---

## Visual Design & Prioritization

| # | Question | Answer |
|---|----------|--------|
| 113 | Mockups/wireframes | None provided - follow Raycast patterns |
| 114 | Visual approach | B) Similar but distinct (familiar patterns, Catppuccin aesthetic) |
| 115 | Must-match behaviors | Clipboard shortcuts, Calculator NLP, Window layouts, Calendar display, Join meeting on Enter |
| 116 | Avoid patterns | None specified |
| 117 | Feature prioritization | **ALL FEATURES ARE MUST-HAVE (Priority 1)** to exit Phase 2 |
| 118 | Defer to Phase 3+ | Only: Clipboard merge (Q18), OCR for images (Q19), Variable assignment in calc (Q25) |
| 119 | Additional requirements | App Sleep feature (stop apps after idle time), Deep scan default ON for uninstall |
| 120 | Availability | As-needed async |

---

## Summary of Key Decisions

### Must-Have Features (All Priority 1)
- **Sprint 4**: Clipboard History (encrypted, full-featured), Calculator (math, currency, units, dates)
- **Sprint 5**: Window Management (cycling, multi-monitor), Quick Links (with Arc import), Calendar (Enter joins meeting), App Management (with App Sleep feature, deep scan default), Sleep Timer, Preferences (4 themes, 14 accents)
- **Sprint 6**: Native Extensions (sandboxed, async API, hot-reload), Custom Commands (streaming output), First-Party Extensions (bundled)

### Deferred to Phase 3+
- Clipboard item merging
- OCR for clipboard images
- Calculator variable assignment
- Interactive custom commands (stdin)

### Notable Customizations from Defaults
1. **Arc browser** added to Quick Links import (in addition to Safari, Chrome, Firefox)
2. **Enter joins meeting** for calendar events with conference links
3. **Deep scan ON by default** for app uninstall (changed from default off)
4. **App Sleep feature** added - stop apps after X idle time (Raycast feature)
5. **All features must-have** - no nice-to-haves, all required for Phase 2 exit
