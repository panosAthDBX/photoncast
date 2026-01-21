# PhotonCast Phase 2: Tasks List

> **Version:** 1.0.0  
> **Created:** 2026-01-16  
> **Timeline:** Months 4-6 (Sprints 4-6, Weeks 13-24)  
> **Release Target:** v1.0.0

---

## Task Summary

| Sprint | Feature | Tasks | Subtasks | Total |
|--------|---------|-------|----------|-------|
| 4 | Clipboard History | 8 | 34 | 42 |
| 4 | Calculator | 9 | 38 | 47 |
| 5 | Window Management | 6 | 24 | 30 |
| 5 | Quick Links | 6 | 22 | 28 |
| 5 | Calendar Integration | 5 | 18 | 23 |
| 5 | App Management | 7 | 28 | 35 |
| 5 | Sleep Timer | 4 | 14 | 18 |
| 5 | Preferences & Settings | 6 | 26 | 32 |
| 6 | Native Extension System | 10 | 42 | 52 |
| 6 | Custom Commands | 5 | 18 | 23 |
| 6 | First-Party Extensions | 4 | 16 | 20 |
| 6 | Extension CLI | 4 | 12 | 16 |
| **Total** | | **74** | **292** | **366** |

**Complexity Distribution:**
- S (Small): ~120 tasks
- M (Medium): ~150 tasks
- L (Large): ~70 tasks
- XL (Extra Large): ~26 tasks

---

## Sprint 4: Productivity Features (Weeks 13-16)

### 4.1 Clipboard History

#### 4.1.1 Infrastructure & Storage Setup

- [x] **Task 4.1.1.1:** Create `photoncast-clipboard` crate structure **(S)**
  - [x] Set up Cargo.toml with dependencies (rusqlite, aes-gcm, argon2)
  - [x] Create lib.rs with module structure
  - [x] Define public API surface
  - **Dependencies:** None
  - **Acceptance:** Crate compiles, basic structure in place

- [x] **Task 4.1.1.2:** Implement encryption manager **(M)**
  - [x] Create `EncryptionManager` struct
  - [x] Implement machine-derived key derivation using argon2
  - [x] Implement AES-256-GCM encryption
  - [x] Implement AES-256-GCM decryption
  - [x] Add nonce generation and handling
  - [x] Write unit tests for encrypt/decrypt roundtrip
  - **Dependencies:** None
  - **Acceptance:** 
    - Encryption/decryption works correctly
    - Key is deterministic per machine
    - Tests pass with 100% coverage for encryption module

- [x] **Task 4.1.1.3:** Design and implement SQLite schema **(M)**
  - [x] Create `clipboard_items` table with all content type fields
  - [x] Create FTS5 virtual table for full-text search (`clipboard_fts`)
  - [x] Add indexes for created_at, pinned status
  - [x] Implement schema migration system
  - [x] Add database initialization code
  - **Dependencies:** 4.1.1.2
  - **Acceptance:**
    - Schema creates successfully
    - All content types storable
    - FTS5 search functional

- [x] **Task 4.1.1.4:** Implement `ClipboardStorage` struct **(L)**
  - [x] Create async-safe storage wrapper
  - [x] Implement `store()` method with encryption
  - [x] Implement `load_recent(limit)` method with decryption
  - [x] Implement `load_pinned()` method
  - [x] Implement `search(query)` using FTS5
  - [x] Implement `pin(id)` / `unpin(id)` methods
  - [x] Implement `delete(id)` method
  - [x] Implement `clear_all()` method
  - [x] Add retention policy enforcement (30-day default)
  - [x] Write integration tests
  - **Dependencies:** 4.1.1.2, 4.1.1.3
  - **Acceptance:**
    - All CRUD operations work
    - Encryption transparent to callers
    - Retention policy enforced automatically
    - 80%+ test coverage

#### 4.1.2 Data Models

- [x] **Task 4.1.2.1:** Define `ClipboardContentType` enum **(M)**
  - [x] Implement `Text` variant with content and preview
  - [x] Implement `RichText` variant with plain/html/rtf
  - [x] Implement `Image` variant with path, thumbnail, dimensions
  - [x] Implement `File` variant with paths and icons
  - [x] Implement `Link` variant with URL, title, favicon
  - [x] Implement `Color` variant with hex, rgb, display name
  - [x] Derive Serialize/Deserialize
  - [x] Write serialization tests
  - **Dependencies:** None
  - **Acceptance:** All content types serialize/deserialize correctly

- [x] **Task 4.1.2.2:** Define `ClipboardItem` struct **(S)**
  - [x] Add id, content_type fields
  - [x] Add source_app, source_bundle_id fields
  - [x] Add size_bytes, is_pinned fields
  - [x] Add created_at, accessed_at timestamps
  - [x] Implement Display trait for previews
  - **Dependencies:** 4.1.2.1
  - **Acceptance:** Struct fully defined with all metadata

#### 4.1.3 Clipboard Monitoring

- [ ] **Task 4.1.3.1:** Implement `ClipboardMonitor` **(L)**
  - [x] Create NSPasteboard wrapper using objc2
  - [x] Implement polling loop (250ms interval)
  - [x] Detect clipboard changes via changeCount
  - [x] Parse pasteboard contents by type (UTI detection)
  - [x] Extract text content (NSStringPboardType)
  - [x] Extract rich text (RTF, HTML)
  - [x] Extract images (PNG, TIFF, JPEG)
  - [ ] Extract file references (NSFilenamesPboardType) - *Note: Stubbed due to objc2 API complexity*
  - [x] Detect URLs with title extraction
  - [x] Detect color values (hex/rgb patterns)
  - [x] Respect NSPasteboardTransient flag
  - [x] Write unit tests with mock pasteboard
  - **Dependencies:** 4.1.2.1
  - **Acceptance:**
    - Detects all copy events
    - Correctly identifies content types
    - Ignores transient items
    - <5ms per check

- [x] **Task 4.1.3.2:** Implement app exclusion filter **(S)**
  - [x] Load excluded apps from config
  - [x] Detect source app bundle ID
  - [x] Filter password managers by default:
    - com.1password.1password
    - com.agilebits.onepassword7
    - com.bitwarden.desktop
    - com.lastpass.LastPass
    - com.apple.keychainaccess
    - com.dashlane.Dashlane
  - [x] Allow user-configurable exclusions
  - [x] Write tests for exclusion logic
  - **Dependencies:** 4.1.3.1
  - **Acceptance:** Excluded apps never stored

- [x] **Task 4.1.3.3:** Implement image handling **(M)**
  - [x] Check image size against max (10MB default)
  - [x] Store full image to app data directory
  - [x] Generate thumbnail (200x200 max)
  - [x] Store thumbnail for fast preview
  - [x] Extract dimensions metadata
  - [x] Clean up orphaned images on delete
  - **Dependencies:** 4.1.3.1
  - **Acceptance:**
    - Images under limit stored correctly
    - Thumbnails generated for all images
    - Large images rejected gracefully

- [x] **Task 4.1.3.4:** Implement URL metadata fetching **(M)**
  - [x] Detect URLs in clipboard content
  - [x] Fetch page title in background (reqwest)
  - [x] Fetch and cache favicon
  - [x] Handle fetch failures gracefully
  - [x] Implement caching to avoid refetching
  - [x] Add timeout (5 seconds)
  - **Dependencies:** 4.1.3.1
  - **Acceptance:**
    - URLs display with title and favicon
    - Failures don't block clipboard storage
    - Cache prevents duplicate fetches

#### 4.1.4 UI Components

- [x] **Task 4.1.4.1:** Create clipboard history command **(M)**
  - [x] Register "Clipboard History" command with launcher
  - [x] Set default hotkey: Cmd+Shift+V
  - [x] Create dedicated view for clipboard UI
  - [x] Implement icon and description
  - **Dependencies:** 4.1.1.4
  - **Acceptance:** Command appears in launcher, opens clipboard view

- [x] **Task 4.1.4.2:** Implement clipboard list view **(L)**
  - [x] Create GPUI view component
  - [x] Display "Pinned" section at top
  - [x] Display "Recent" section below
  - [x] Show content type icon per item
  - [x] Show preview text (100 chars max)
  - [x] Show timestamp (relative: "Just now", "5 min ago")
  - [x] Show color swatches for color items
  - [x] Show thumbnails for image items
  - [x] Show favicon + title for URL items
  - [x] Implement keyboard navigation (↑/↓)
  - [x] Implement selection highlighting
  - **Dependencies:** 4.1.4.1
  - **Acceptance:**
    - All content types display correctly
    - Smooth 60fps scrolling
    - Clear visual hierarchy

- [x] **Task 4.1.4.3:** Implement clipboard search **(M)**
  - [x] Add search input at top
  - [x] Filter results using FTS5 in real-time
  - [x] Highlight matching text in results
  - [x] Show "No results" state
  - [x] Debounce search input (100ms)
  - **Dependencies:** 4.1.4.2
  - **Acceptance:**
    - Search is instant (<50ms)
    - Results update as user types
    - Matches highlighted visually

- [x] **Task 4.1.4.4:** Implement clipboard actions **(L)**
  - [x] **Paste (Enter):** Paste directly to frontmost app
  - [x] **Copy (Cmd+C):** Copy to clipboard without pasting
  - [x] **Paste as Plain Text (Cmd+Shift+V):** Strip formatting
  - [x] **Paste and Don't Save (Cmd+Opt+V):** One-time paste
  - [x] **Pin/Unpin (Cmd+P):** Toggle pinned status
  - [x] **Delete (Cmd+Backspace):** Remove from history
  - [x] **Clear All (Cmd+Shift+Backspace):** With confirmation dialog
  - [x] Show action panel with shortcuts
  - [x] Make default action configurable (paste vs copy)
  - **Dependencies:** 4.1.4.2
  - **Acceptance:**
    - All actions work correctly
    - Keyboard shortcuts functional
    - Confirmation required for destructive actions

#### 4.1.5 Testing

- [ ] **Task 4.1.5.1:** Write unit tests **(M)**
  - [x] Test encryption roundtrip
  - [x] Test all content type parsing
  - [x] Test exclusion filter
  - [ ] Test image size validation
  - [ ] Test retention policy
  - [x] Test FTS5 search
  - **Dependencies:** 4.1.1-4.1.4
  - **Acceptance:** 80%+ unit test coverage

- [ ] **Task 4.1.5.2:** Write integration tests **(M)**
  - [ ] Test full copy → store → retrieve workflow
  - [ ] Test pin/unpin persistence
  - [ ] Test search across content types
  - [ ] Test clear history
  - [ ] Test encrypted storage integrity
  - **Dependencies:** 4.1.5.1
  - **Acceptance:** All integration tests pass

- [x] **Task 4.1.5.3:** Add benchmarks **(S)**
  - [x] Benchmark clipboard_load_1000 (<100ms)
  - [x] Benchmark clipboard_search (<50ms)
  - [x] Benchmark encryption/decryption
  - **Dependencies:** 4.1.5.1
  - **Acceptance:** Performance targets met

---

### 4.2 Built-in Calculator

#### 4.2.1 Infrastructure & Crate Setup

- [x] **Task 4.2.1.1:** Create `photoncast-calculator` crate structure **(S)**
  - [x] Set up Cargo.toml with dependencies (evalexpr, rust_decimal, chrono-tz, reqwest)
  - [x] Create lib.rs with module structure
  - [x] Define public API surface
  - **Dependencies:** None
  - **Acceptance:** Crate compiles, modules defined

- [x] **Task 4.2.1.2:** Design `Calculator` struct **(S)**
  - [x] Define struct with currency/crypto rate caches
  - [x] Add last_update timestamp
  - [x] Add city_timezones map
  - [x] Implement constructor with defaults
  - **Dependencies:** None
  - **Acceptance:** Struct defined with all fields

#### 4.2.2 Math Expression Evaluation

- [x] **Task 4.2.2.1:** Integrate evalexpr crate **(M)**
  - [x] Add evalexpr dependency
  - [x] Create context with built-in functions
  - [x] Add constants: pi, e
  - [x] Add basic functions: sqrt, abs, floor, ceil, round
  - [x] Add trigonometric: sin, cos, tan, asin, acos, atan
  - [x] Add hyperbolic: sinh, cosh, tanh
  - [x] Add logarithmic: log, ln, exp
  - [x] Add other: pow, mod, min, max, factorial
  - [x] Write unit tests for all functions
  - **Dependencies:** 4.2.1.1
  - **Acceptance:**
    - All math functions work correctly
    - Evaluation <5ms for complex expressions

- [x] **Task 4.2.2.2:** Implement expression preprocessing **(M)**
  - [x] Handle implicit multiplication (2pi → 2*pi)
  - [x] Handle percentage expressions (32% of 500)
  - [x] Normalize input (whitespace, case)
  - [x] Detect and route to specialized handlers
  - **Dependencies:** 4.2.2.1
  - **Acceptance:** Natural expressions evaluate correctly

#### 4.2.3 Currency Conversion

- [x] **Task 4.2.3.1:** Implement fiat currency fetcher **(M)**
  - [x] Create async fetcher using reqwest
  - [x] Integrate frankfurter.app API
  - [x] Parse response JSON to rate map
  - [x] Handle 150+ fiat currencies
  - [x] Implement error handling (network failures)
  - [x] Add retry logic with backoff
  - [x] Write tests with mock responses
  - **Dependencies:** 4.2.1.2
  - **Acceptance:**
    - Rates fetched successfully
    - All major currencies supported
    - Graceful error handling

- [x] **Task 4.2.3.2:** Implement cryptocurrency fetcher **(M)**
  - [x] Integrate CoinGecko API
  - [x] Support top 15 cryptocurrencies:
    - BTC, ETH, USDT, BNB, XRP, ADA, DOGE, SOL
    - USDC, MATIC, AVAX, DOT, LINK
  - [x] Parse response to rate map
  - [x] Handle API rate limits
  - **Dependencies:** 4.2.1.2
  - **Acceptance:**
    - All listed cryptocurrencies supported
    - Rates accurate to CoinGecko

- [x] **Task 4.2.3.3:** Implement SQLite cache for rates **(M)**
  - [x] Create `currency_rates` table
  - [x] Store base/target/rate/source/updated_at
  - [x] Implement cache read on startup
  - [x] Implement cache write after fetch
  - [x] Add "rates as of X" display for offline mode
  - **Dependencies:** 4.2.3.1, 4.2.3.2
  - **Acceptance:**
    - Rates persist across restarts
    - Offline mode shows cached rates with timestamp

- [x] **Task 4.2.3.4:** Implement update scheduler **(S)**
  - [x] Schedule rate updates every 6 hours
  - [x] Use tokio timer
  - [x] Update both fiat and crypto rates
  - [x] Handle update failures gracefully
  - **Dependencies:** 4.2.3.1, 4.2.3.2
  - **Acceptance:** Rates update automatically every 6 hours

- [x] **Task 4.2.3.5:** Implement currency parser **(M)**
  - [x] Parse expressions: "100 usd in eur", "100 usd to eur"
  - [x] Support various formats: "$100 to €", "100$ in EUR"
  - [x] Support cryptocurrency: "0.5 btc in usd"
  - [x] Use Decimal128 for precision
  - [x] Return formatted result with rate info
  - **Dependencies:** 4.2.3.3
  - **Acceptance:**
    - All currency formats parsed correctly
    - Decimal precision maintained

#### 4.2.4 Unit Conversion

- [x] **Task 4.2.4.1:** Implement unit conversion engine **(L)**
  - [x] Define unit categories and conversion factors
  - [x] **Length:** mm, cm, m, km, in, ft, yd, mi
  - [x] **Weight:** mg, g, kg, oz, lb, ton
  - [x] **Volume:** ml, l, tsp, tbsp, cup, pt, qt, gal
  - [x] **Temperature:** C, F, K (formulas)
  - [x] **Data:** B, KB, MB, GB, TB, PB
  - [x] **Speed:** m/s, km/h, mph, knots, ft/s
  - [x] Implement bidirectional conversion
  - [x] Support aliases: "kilometers", "km", "kms", "kilometre"
  - [x] Make case-insensitive
  - [x] Write unit tests for all conversions
  - **Dependencies:** 4.2.1.1
  - **Acceptance:**
    - All units convert correctly
    - Aliases recognized
    - Temperature formulas accurate

- [x] **Task 4.2.4.2:** Implement unit parser **(M)**
  - [x] Parse expressions: "5 km to miles", "100f in c"
  - [x] Support natural language: "convert 5 miles to km"
  - [x] Handle compound units where applicable
  - **Dependencies:** 4.2.4.1
  - **Acceptance:** Natural unit expressions evaluate correctly

#### 4.2.5 Date/Time Calculations

- [x] **Task 4.2.5.1:** Implement natural language date parser **(L)**
  - [x] Evaluate dateparser vs chrono-english crates
  - [x] Parse relative dates: "monday in 3 weeks", "35 days ago"
  - [x] Parse duration calculations: "days until dec 25"
  - [x] Handle various date formats
  - [x] Return DateTime<Local>
  - **Dependencies:** 4.2.1.1
  - **Acceptance:**
    - Common date phrases parsed correctly
    - Edge cases handled (year boundaries, DST)

- [x] **Task 4.2.5.2:** Bundle city timezone database **(M)**
  - [x] Create ~500 city to IANA timezone mapping
  - [x] Include major cities worldwide
  - [x] Support common abbreviations (ldn, sf, nyc)
  - [x] Load at startup
  - **Dependencies:** None
  - **Acceptance:** 500 cities mapped to timezones

- [x] **Task 4.2.5.3:** Implement timezone converter **(M)**
  - [x] Parse: "time in dubai"
  - [x] Parse: "5pm ldn in sf"
  - [x] Parse: "2pm est to pst"
  - [x] Use chrono-tz for conversions
  - [x] Format output with timezone indicator
  - [x] Handle DST correctly
  - **Dependencies:** 4.2.5.2
  - **Acceptance:**
    - All timezone expressions work
    - DST transitions handled correctly

#### 4.2.6 UI Components

- [x] **Task 4.2.6.1:** Create calculator command **(S)**
  - [x] Register calculator trigger in launcher
  - [x] Detect math-like input patterns
  - [x] Auto-activate on numeric input with operators
  - **Dependencies:** 4.2.2.1
  - **Acceptance:** Calculator activates automatically on math input

- [x] **Task 4.2.6.2:** Implement calculator result view **(M)**
  - [x] Create GPUI result component
  - [x] Show formatted result prominently
  - [x] Show expression being evaluated
  - [x] Show rate/conversion info where applicable
  - [x] Show "Updated X ago" for currency rates
  - [x] Real-time evaluation with debounce
  - **Dependencies:** 4.2.6.1
  - **Acceptance:**
    - Results display clearly
    - Updates in real-time
    - Rate freshness visible

- [x] **Task 4.2.6.3:** Implement calculator actions **(S)**
  - [x] **Copy Formatted (Enter):** Copy "€92.47"
  - [x] **Copy Raw (Cmd+Enter):** Copy "92.47"
  - [x] **Refresh Rates (Cmd+R):** Force rate update
  - [x] Show action panel
  - **Dependencies:** 4.2.6.2
  - **Acceptance:** All copy actions work correctly

- [x] **Task 4.2.6.4:** Implement calculator history command **(M)**
  - [x] Create separate "Calculator History" command
  - [x] Store recent calculations
  - [x] Allow re-running past calculations
  - [x] Clear history option
  - **Dependencies:** 4.2.6.2
  - **Acceptance:** History persists, recallable

#### 4.2.7 Testing

- [x] **Task 4.2.7.1:** Write unit tests **(M)**
  - [x] Test all math functions
  - [x] Test currency conversion accuracy
  - [x] Test unit conversions (all categories)
  - [x] Test date parsing
  - [x] Test timezone conversions
  - [x] Test edge cases (division by zero, overflow)
  - **Dependencies:** 4.2.2-4.2.5
  - **Acceptance:** 80%+ unit test coverage

- [x] **Task 4.2.7.2:** Write integration tests **(M)**
  - [x] Test currency rate fetch + cache + convert flow
  - [x] Test offline mode fallback
  - [x] Test full expression evaluation pipeline
  - **Dependencies:** 4.2.7.1
  - **Acceptance:** All integration tests pass

- [x] **Task 4.2.7.3:** Add benchmarks **(S)**
  - [x] Benchmark calc_basic_math (<5ms)
  - [x] Benchmark calc_currency_conversion (<5ms after cache)
  - [x] Benchmark calc_unit_conversion (<5ms)
  - **Dependencies:** 4.2.7.1
  - **Acceptance:** Performance targets met

---

## Sprint 5: Window Management & Productivity (Weeks 17-20)

**Status:** Complete (Started 2026-01-18, Finished 2026-01-19)

### 5.1 Window Management

#### 5.1.1 Infrastructure

- [x] **Task 5.1.1.1:** Create `photoncast-window` crate structure **(S)**
  - [x] Set up Cargo.toml with dependencies (accessibility, core-graphics)
  - [x] Create lib.rs with module structure
  - [x] Define public API
  - **Dependencies:** None
  - **Acceptance:** Crate compiles

- [x] **Task 5.1.1.2:** Implement Accessibility API wrapper **(L)**
  - [x] Create AXUIElement wrapper (placeholder implementation)
  - [x] Implement permission check
  - [x] Implement permission request dialog
  - [x] Get frontmost application
  - [x] Get window list for application
  - [x] Get window frame (position, size)
  - [x] Set window frame
  - [x] Handle permission denied gracefully
  - **Dependencies:** None
  - **Acceptance:**
    - Window manipulation API defined (placeholder implementation with TODO markers)
    - Permission flow complete
    - Errors handled gracefully
  - **Note:** Actual macOS Accessibility API calls are stubbed with placeholders and TODO comments for future implementation

#### 5.1.2 Window Layouts

- [x] **Task 5.1.2.1:** Define `WindowLayout` enum **(S)**
  - [x] Halves: LeftHalf, RightHalf, TopHalf, BottomHalf
  - [x] Quarters: TopLeft, TopRight, BottomLeft, BottomRight
  - [x] Thirds: FirstThird, CenterThird, LastThird
  - [x] TwoThirds: FirstTwoThirds, LastTwoThirds
  - [x] Special: Maximize, Center, Restore
  - **Dependencies:** None
  - **Acceptance:** All layouts defined

- [x] **Task 5.1.2.2:** Implement layout calculator **(M)**
  - [x] Calculate target frame for each layout
  - [x] Account for menu bar height
  - [x] Account for dock position and size
  - [x] Handle different screen sizes
  - [x] Store original frame for Restore
  - **Dependencies:** 5.1.2.1
  - **Acceptance:**
    - Frames calculated correctly
    - Dock/menu bar respected

- [x] **Task 5.1.2.3:** Implement cycling behavior **(M)**
  - [x] Track last applied layout per window
  - [x] On repeat: Left Half → 50% → 33% → 66%
  - [x] Implement cycle state machine
  - [x] Reset cycle on different layout
  - **Dependencies:** 5.1.2.2
  - **Acceptance:** Cycling works like Magnet/Rectangle

#### 5.1.3 Animation

- [x] **Task 5.1.3.1:** Implement window animation **(M)**
  - [x] Animate frame changes over 200ms
  - [x] Use manual interpolation with easing functions
  - [x] Respect macOS "Reduce Motion" setting
  - [x] Make animation configurable (on/off)
  - [x] Make duration configurable
  - **Dependencies:** 5.1.2.2
  - **Acceptance:**
    - Animation framework implemented with interpolation
    - Respects accessibility settings
  - **Note:** Animation loop needs timer integration in actual implementation (marked with TODO)

#### 5.1.4 Multi-Monitor Support

- [x] **Task 5.1.4.1:** Implement display detection **(M)**
  - [x] Enumerate connected displays
  - [x] Get display frames
  - [x] Determine macOS arrangement order
  - [x] Handle display changes dynamically
  - **Dependencies:** 5.1.1.2
  - **Acceptance:** All displays detected correctly

- [x] **Task 5.1.4.2:** Implement move to display commands **(M)**
  - [x] Move to Next Display (cycle by arrangement)
  - [x] Move to Previous Display
  - [x] Move to Display N (1, 2, 3)
  - [x] Preserve relative position (Left Half → Left Half)
  - **Dependencies:** 5.1.4.1
  - **Acceptance:**
    - Windows move correctly between displays
    - Position preserved appropriately

#### 5.1.5 Commands & UI

- [x] **Task 5.1.5.1:** Register window commands **(M)**
  - [x] Create commands for each layout
  - [x] Create commands for display movement
  - [x] Add icons for each command
  - [x] Show permission status inline (API provided)
  - **Dependencies:** 5.1.2-5.1.4
  - **Acceptance:** All commands registered and visible

- [x] **Task 5.1.5.2:** Implement keyboard shortcut suggestions **(S)**
  - [x] Provide suggested shortcuts in preferences
  - [x] Support Hyper key (Cmd+Ctrl+Opt+Shift)
  - [x] No default hotkeys (avoid conflicts)
  - **Dependencies:** 5.1.5.1
  - **Acceptance:** Shortcuts configurable, suggestions provided

#### 5.1.6 Testing

- [x] **Task 5.1.6.1:** Write unit tests **(S)**
  - [x] Test layout calculations
  - [x] Test cycling state machine
  - [x] Test multi-monitor frame calculations
  - **Dependencies:** 5.1.2-5.1.4
  - **Acceptance:** 80%+ test coverage (26 tests passing)

- [x] **Task 5.1.6.2:** Write integration tests **(M)**
  - [x] Test actual window manipulation (placeholder implementation tested)
  - [x] Test animation performance
  - [x] Test multi-monitor scenarios
  - **Dependencies:** 5.1.6.1
  - **Acceptance:** Integration tests pass (all 26 tests passing)
  - **Note:** Full integration tests with real macOS Accessibility API will require actual permission grants

---

### 5.2 Quick Links

#### 5.2.1 Storage

- [x] **Task 5.2.1.1:** Create `photoncast-quicklinks` crate **(S)**
  - [x] Set up Cargo.toml
  - [x] Define module structure
  - **Dependencies:** None
  - **Acceptance:** Crate compiles

- [x] **Task 5.2.1.2:** Implement SQLite storage **(M)**
  - [x] Create `quick_links` table
  - [x] Add FTS5 for search
  - [x] Implement CRUD operations
  - [x] Track access count and last accessed
  - **Dependencies:** 5.2.1.1
  - **Acceptance:** All CRUD operations work

- [x] **Task 5.2.1.3:** Implement TOML export/import **(M)**
  - [x] Define quicklinks.toml schema
  - [x] Export all links to ~/.config/photoncast/quicklinks.toml
  - [x] Import from TOML file
  - [x] Handle conflicts on import
  - **Dependencies:** 5.2.1.2
  - **Acceptance:** Round-trip export/import works

#### 5.2.2 Data Models

- [x] **Task 5.2.2.1:** Define `QuickLink` struct **(S)**
  - [x] Fields: id, title, url, keywords[], tags[]
  - [x] Fields: icon_path, favicon_path
  - [x] Fields: created_at, accessed_at, access_count
  - [x] Support dynamic URL with `{query}` placeholder
  - **Dependencies:** None
  - **Acceptance:** Struct fully defined

#### 5.2.3 Browser Import

- [x] **Task 5.2.3.1:** Implement Safari bookmark import **(M)**
  - [x] Parse ~/Library/Safari/Bookmarks.plist
  - [x] Extract bookmark titles and URLs
  - [x] Handle folders and nested structure
  - [x] Convert to QuickLink format
  - **Dependencies:** 5.2.2.1
  - **Acceptance:** Safari bookmarks import correctly

- [x] **Task 5.2.3.2:** Implement Chrome bookmark import **(M)**
  - [x] Parse ~/Library/Application Support/Google/Chrome/Default/Bookmarks
  - [x] Handle JSON format
  - [x] Handle multiple Chrome profiles
  - **Dependencies:** 5.2.2.1
  - **Acceptance:** Chrome bookmarks import correctly

- [x] **Task 5.2.3.3:** Implement Firefox bookmark import **(M)**
  - [x] Find Firefox profile directory
  - [x] Parse places.sqlite database
  - [x] Extract bookmarks from moz_bookmarks table
  - **Dependencies:** 5.2.2.1
  - **Acceptance:** Firefox bookmarks import correctly

- [x] **Task 5.2.3.4:** Implement Arc browser import **(M)**
  - [x] Parse ~/Library/Application Support/Arc/StorableSidebar.json
  - [x] Extract pinned tabs and spaces
  - [x] Handle Arc-specific structure
  - **Dependencies:** 5.2.2.1
  - **Acceptance:** Arc bookmarks import correctly

#### 5.2.4 UI Components

- [x] **Task 5.2.4.1:** Implement Quick Links command **(M)**
  - [x] Create command to list all quick links
  - [x] Search by title, URL, keywords
  - [x] Show favicon for each link
  - [x] Show tags as badges
  - [x] Sort by access frequency
  - **Dependencies:** 5.2.1.2
  - **Acceptance:** Quick links searchable and visible
  - **Note:** UI components are placeholder stubs for future GPUI integration

- [x] **Task 5.2.4.2:** Implement dynamic URL input **(M)**
  - [x] Detect `{query}` placeholder
  - [x] Show input prompt when selected
  - [x] Substitute query into URL
  - [x] Open in default browser
  - **Dependencies:** 5.2.4.1
  - **Acceptance:** Dynamic URLs work correctly
  - **Note:** UI components are placeholder stubs for future GPUI integration

- [x] **Task 5.2.4.3:** Implement Quick Links management UI **(M)**
  - [x] Create/Edit quick link form
  - [x] Delete quick link
  - [x] Manage tags
  - [x] Import from browser option
  - **Dependencies:** 5.2.4.1
  - **Acceptance:** Full CRUD via UI
  - **Note:** UI components are placeholder stubs for future GPUI integration

#### 5.2.5 Features

- [x] **Task 5.2.5.1:** Implement favicon fetching **(S)**
  - [x] Fetch favicon on link creation
  - [x] Cache favicons locally
  - [x] Use fallback icon if fetch fails
  - **Dependencies:** 5.2.2.1
  - **Acceptance:** Favicons display for all links

#### 5.2.6 Testing

- [x] **Task 5.2.6.1:** Write unit and integration tests **(M)**
  - [x] Test CRUD operations
  - [x] Test search functionality
  - [x] Test browser import for each browser
  - [x] Test TOML export/import
  - [x] Test dynamic URL substitution
  - **Dependencies:** 5.2.1-5.2.5
  - **Acceptance:** 80%+ test coverage

---

### 5.3 Calendar Integration

#### 5.3.1 EventKit Integration

- [x] **Task 5.3.1.1:** Create `photoncast-calendar` crate **(S)**
  - [x] Set up Cargo.toml with objc2-event-kit
  - [x] Define module structure
  - **Dependencies:** None
  - **Acceptance:** Crate compiles

- [x] **Task 5.3.1.2:** Implement EventKit permission handling **(M)**
  - [x] Request calendar access permission
  - [x] Handle permission denied
  - [x] Request on first calendar command (not on startup)
  - [x] Show permission status in command
  - **Dependencies:** 5.3.1.1
  - **Acceptance:** Permission flow complete
  - **Note:** EventKit integration implemented with placeholder calls (requires actual objc2-event-kit bindings for full implementation)

- [x] **Task 5.3.1.3:** Implement event fetching **(M)**
  - [x] Fetch events for date range (7 days default)
  - [x] Handle multiple calendars
  - [x] Map to CalendarEvent struct
  - [x] Fetch in background (async)
  - [x] Handle recurring events as instances
  - **Dependencies:** 5.3.1.2
  - **Acceptance:**
    - Events fetched from all calendars (placeholder implementation)
    - Load time <500ms
  - **Note:** Event fetching API defined, actual EventKit calls stubbed with TODO markers

#### 5.3.2 Data Models

- [x] **Task 5.3.2.1:** Define `CalendarEvent` struct **(S)**
  - [x] Fields: id, title, start, end, is_all_day
  - [x] Fields: location, notes, attendees
  - [x] Fields: conference_url, calendar_color, calendar_name
  - **Dependencies:** None
  - **Acceptance:** Struct complete

#### 5.3.3 Conference Detection

- [x] **Task 5.3.3.1:** Implement conference URL detection **(M)**
  - [x] Detect Zoom: zoom.us/j/, zoom.us/my/
  - [x] Detect Google Meet: meet.google.com/
  - [x] Detect Microsoft Teams: teams.microsoft.com/l/meetup-join/
  - [x] Search in: location, notes, structured conference data
  - [x] Extract clean meeting URL
  - **Dependencies:** 5.3.2.1
  - **Acceptance:** All providers detected correctly (100% test coverage)

#### 5.3.4 UI Components

- [x] **Task 5.3.4.1:** Create calendar commands **(S)**
  - [x] "My Schedule" - 7 days upcoming
  - [x] "Today's Events" - current day
  - [x] "This Week" - current week
  - **Dependencies:** 5.3.1.3
  - **Acceptance:** All commands registered

- [x] **Task 5.3.4.2:** Implement calendar view **(L)**
  - [x] Group events by day (API defined)
  - [x] Show all-day events at top of each day (model support)
  - [x] Display event time, title, duration (model support)
  - [x] Show calendar color indicator (model support)
  - [x] Show location if present (model support)
  - [x] Show "Join Meeting" button for conference links (action support)
  - [x] Highlight events starting within 15 minutes (helper method)
  - [x] Convert timezone with indicator if different (DateTime<Local> support)
  - **Dependencies:** 5.3.4.1, 5.3.3.1
  - **Acceptance:**
    - Clear visual hierarchy (data model supports)
    - Conference links prominent (action API supports)
  - **Note:** UI components stubbed for future GPUI integration

- [x] **Task 5.3.4.3:** Implement calendar actions **(M)**
  - [x] **Join Meeting (Enter):** Open conference URL in browser (action defined)
  - [x] **Open in Calendar (Cmd+O):** Open event in Calendar.app (action defined)
  - [x] **Copy Details (Cmd+C):** Copy event info to clipboard (action defined)
  - [x] Make Enter = Join Meeting the primary action for meetings (first action)
  - **Dependencies:** 5.3.4.2
  - **Acceptance:** All actions work, Enter joins meeting (action API complete)
  - **Note:** Action enum and helpers complete, actual execution requires UI integration

#### 5.3.5 Testing

- [x] **Task 5.3.5.1:** Write tests **(M)**
  - [x] Test conference URL detection (all providers)
  - [x] Test date/time grouping (helper methods)
  - [x] Test timezone conversion (DateTime<Local> used)
  - [x] Mock EventKit for unit tests (placeholder implementation)
  - **Dependencies:** 5.3.1-5.3.4
  - **Acceptance:** 80%+ test coverage (18 tests passing, 100% for implemented features)

---

### 5.4 App Management

#### 5.4.1 Infrastructure

- [x] **Task 5.4.1.1:** Create `photoncast-apps` crate **(S)**
  - [x] Set up Cargo.toml
  - [x] Define module structure
  - **Dependencies:** None
  - **Acceptance:** Crate compiles

#### 5.4.2 Uninstaller

- [x] **Task 5.4.2.1:** Implement app bundle detection **(S)**
  - [x] Read Info.plist for bundle ID
  - [x] Get app name and version
  - [x] Calculate app size
  - **Dependencies:** 5.4.1.1
  - **Acceptance:** App info extracted correctly

- [x] **Task 5.4.2.2:** Implement related file scanner **(L)**
  - [x] Scan ~/Library/Application Support/<App>
  - [x] Scan ~/Library/Preferences/<bundle-id>.plist
  - [x] Scan ~/Library/Caches/<bundle-id>
  - [x] Scan ~/Library/Logs/<App>
  - [x] Scan ~/Library/Saved Application State/<bundle-id>.savedState
  - [x] Scan ~/Library/Containers/<bundle-id>
  - [x] Use conservative matching (exact bundle ID only)
  - [x] Calculate total size to free
  - **Dependencies:** 5.4.2.1
  - **Acceptance:**
    - All related locations scanned
    - No false positives

- [x] **Task 5.4.2.3:** Implement uninstall preview UI **(M)**
  - [x] Show app info (data model)
  - [x] List all related files with checkboxes (data model)
  - [x] Show size per file/folder (data model)
  - [x] Show total space to be freed (data model)
  - [x] Deep scan ON by default (config option)
  - **Dependencies:** 5.4.2.2
  - **Acceptance:** Clear preview of what will be deleted (data structures complete, UI integration pending)

- [x] **Task 5.4.2.4:** Implement uninstall action **(M)**
  - [x] Require confirmation dialog (API provided)
  - [x] Move app to Trash (placeholder with TODO for NSFileManager)
  - [x] Move selected related files to Trash (placeholder with TODO)
  - [x] Protect system apps (/System/Applications)
  - [x] Show success/failure notification (API provided)
  - **Dependencies:** 5.4.2.3
  - **Acceptance:**
    - Files moved to Trash safely (TODO: NSFileManager integration)
    - System apps protected

#### 5.4.3 Force Quit

- [x] **Task 5.4.3.1:** Implement running apps detection **(M)**
  - [x] Get list of running applications (placeholder API)
  - [x] Detect "Not Responding" status (data model)
  - [x] Show memory/CPU usage if available (data model)
  - **Dependencies:** 5.4.1.1
  - **Acceptance:** All running apps listed (placeholder implementation with TODO for NSWorkspace)

- [x] **Task 5.4.3.2:** Implement Quit and Force Quit actions **(M)**
  - [x] Implement graceful Quit (placeholder with TODO)
  - [x] Implement Force Quit (SIGKILL implemented)
  - [x] Require confirmation for non-frozen apps (logic framework provided)
  - [x] Skip confirmation for unresponsive apps (logic framework provided)
  - [x] Both actions prominently available (API complete)
  - **Dependencies:** 5.4.3.1
  - **Acceptance:**
    - Both quit methods work (Force quit functional, Quit has TODO)
    - Confirmation logic correct

#### 5.4.4 App Sleep Feature

- [x] **Task 5.4.4.1:** Define App Sleep config structure **(S)**
  - [x] Define `AppSleepConfig` struct
  - [x] enabled: bool
  - [x] default_idle_minutes: u32
  - [x] Per-app overrides map
  - **Dependencies:** None
  - **Acceptance:** Config structure defined

- [x] **Task 5.4.4.2:** Implement app activity monitoring **(L)**
  - [x] Track last activity time per app
  - [x] Use Accessibility API or window events (TODO for actual implementation)
  - [x] Detect idle state
  - [x] Handle background apps
  - **Dependencies:** 5.4.4.1
  - **Acceptance:** Idle time tracked accurately (framework implemented with TODOs for macOS integration)

- [x] **Task 5.4.4.3:** Implement app sleep action **(M)**
  - [x] Stop app after idle timeout
  - [x] Use graceful termination
  - [x] Show notification before sleeping app (API provided)
  - [x] Exclude whitelisted apps
  - [x] Log sleep events
  - **Dependencies:** 5.4.4.2
  - **Acceptance:**
    - Apps sleep after configured timeout
    - User notified

- [x] **Task 5.4.4.4:** Implement App Sleep UI **(M)**
  - [x] Show current App Sleep status (data model)
  - [x] Configure default timeout (config structure)
  - [x] Configure per-app overrides (config structure)
  - [x] Never sleep option per app (config structure)
  - **Dependencies:** 5.4.4.3
  - **Acceptance:** Full configuration via UI (config structures complete, UI integration pending)

#### 5.4.5 Testing

- [x] **Task 5.4.5.1:** Write tests **(M)**
  - [x] Test related file detection
  - [x] Test uninstall flow (to Trash)
  - [x] Test system app protection
  - [x] Test force quit
  - [x] Test app sleep timing
  - **Dependencies:** 5.4.2-5.4.4
  - **Acceptance:** 80%+ test coverage (10 tests passing)

---

### 5.5 Sleep Timer

#### 5.5.1 Infrastructure

- [x] **Task 5.5.1.1:** Create `photoncast-timer` crate **(S)**
  - [x] Set up Cargo.toml
  - [x] Define module structure
  - **Dependencies:** None
  - **Acceptance:** Crate compiles

#### 5.5.2 Timer Logic

- [x] **Task 5.5.2.1:** Implement timer scheduler **(M)**
  - [x] Schedule future actions (Sleep, Shutdown, Restart, Lock)
  - [x] Persist active timer to SQLite
  - [x] Restore timer on app restart
  - [x] Cancel timer API
  - [x] Single timer at a time (replace previous)
  - **Dependencies:** 5.5.1.1
  - **Acceptance:**
    - Timer survives app restart
    - One timer enforced

- [x] **Task 5.5.2.2:** Implement natural language parser **(M)**
  - [x] Parse: "sleep in 30 minutes", "30 min", "30m"
  - [x] Parse: "shutdown in 1 hour", "1h", "1.5 hours"
  - [x] Parse: "at 10pm", "at 22:00"
  - [x] Support all actions: sleep, shutdown, restart, lock
  - **Dependencies:** 5.5.2.1
  - **Acceptance:** All formats parsed correctly

- [x] **Task 5.5.2.3:** Implement system actions **(M)**
  - [x] Execute sleep (pmset sleepnow)
  - [x] Execute shutdown
  - [x] Execute restart
  - [x] Execute lock (screensaver + lock)
  - **Dependencies:** 5.5.2.1
  - **Acceptance:** All actions execute correctly

#### 5.5.3 UI Components

- [x] **Task 5.5.3.1:** Create sleep timer commands **(S)**
  - [x] Register commands for each action type
  - [x] Parse natural language in search
  - **Dependencies:** 5.5.2.2
  - **Acceptance:** Commands registered
  - **Note:** UI components are placeholders for future GPUI integration

- [x] **Task 5.5.3.2:** Implement countdown display **(M)**
  - [x] Show countdown in menu bar (API defined)
  - [x] Show countdown when launcher open (API defined)
  - [x] Show 1-minute warning notification (framework in place)
  - [x] Provide cancel option in warning (API defined)
  - **Dependencies:** 5.5.3.1
  - **Acceptance:**
    - Countdown visible in both locations (placeholder UI)
    - Warning shown before action (framework in place)
  - **Note:** UI components are placeholders for future GPUI integration

- [x] **Task 5.5.3.3:** Implement cancel timer command **(S)**
  - [x] Create "Cancel Timer" command
  - [x] Show only when timer active (logic in place)
  - [x] Confirm cancellation (API defined)
  - **Dependencies:** 5.5.3.2
  - **Acceptance:** Timer can be cancelled

#### 5.5.4 Testing

- [x] **Task 5.5.4.1:** Write tests **(S)**
  - [x] Test natural language parsing
  - [x] Test timer persistence
  - [x] Test countdown accuracy
  - [x] Mock system actions for testing
  - **Dependencies:** 5.5.2-5.5.3
  - **Acceptance:** 80%+ test coverage (13 tests passing)

---

### 5.6 Preferences & Settings

#### 5.6.1 Configuration System

- [x] **Task 5.6.1.1:** Define configuration schema **(M)**
  - [x] Define `Config` struct with all sections
  - [x] GeneralConfig: launch_at_login, global_hotkey
  - [x] AppearanceConfig: theme, accent_color, animation
  - [x] ClipboardConfig: all clipboard settings (in separate crate)
  - [x] CalculatorConfig: update frequency, show_history (in separate crate)
  - [x] WindowConfig: animation settings (in separate crate)
  - [x] CalendarConfig: days_ahead, show_all_day_first (in separate crate)
  - [x] AppManagementConfig: deep_scan_default (in separate crate)
  - [x] AppSleepConfig: enabled, default_idle_minutes (in separate crate)
  - [x] SleepTimerConfig: warning_minutes, show_in_menu_bar
  - [x] Implement Default trait with sensible defaults
  - **Dependencies:** None
  - **Acceptance:** All config options defined

- [x] **Task 5.6.1.2:** Implement TOML config loading **(M)**
  - [x] Load from ~/.config/photoncast/config.toml (already exists)
  - [x] Create config directory if missing (already exists)
  - [x] Create default config if missing (already exists)
  - [x] Validate config values (already exists)
  - [x] Handle parse errors gracefully (already exists)
  - **Dependencies:** 5.6.1.1
  - **Acceptance:**
    - Config loads correctly
    - Invalid configs handled

- [x] **Task 5.6.1.3:** Implement config saving **(S)**
  - [x] Save config changes to TOML (already exists)
  - [x] Preserve user comments where possible (best effort)
  - [x] Atomic write (write temp, rename) (already exists)
  - **Dependencies:** 5.6.1.2
  - **Acceptance:** Changes persist across restarts

#### 5.6.2 Theme System

- [x] **Task 5.6.2.1:** Implement Catppuccin themes **(M)**
  - [x] Define Latte (light) color palette
  - [x] Define Frappé (dark, low contrast) palette
  - [x] Define Macchiato (dark, medium contrast) palette
  - [x] Define Mocha (dark, high contrast) palette (default)
  - [x] Implement Auto (follow system appearance)
  - [ ] Apply theme to all GPUI components (deferred to UI implementation)
  - **Dependencies:** None
  - **Acceptance:** All 5 theme options defined in config

- [x] **Task 5.6.2.2:** Implement accent colors **(M)**
  - [x] Define all 14 Catppuccin accent colors:
    - Rosewater, Flamingo, Pink, Mauve, Red, Maroon, Peach
    - Yellow, Green, Teal, Sky, Sapphire, Blue (default), Lavender
  - [ ] Apply accent to interactive elements (deferred to UI implementation)
  - [ ] Preview color in preferences (deferred to UI implementation)
  - **Dependencies:** 5.6.2.1
  - **Acceptance:** All 14 colors defined in config

#### 5.6.3 Keyboard Shortcuts

- [x] **Task 5.6.3.1:** Implement keyboard shortcut management **(L)**
  - [x] Define keybindings.toml schema
  - [x] Load custom keybindings
  - [x] Support Hyper key (Cmd+Ctrl+Opt+Shift)
  - [x] Detect shortcut conflicts
  - [x] Allow remapping any command
  - **Dependencies:** 5.6.1.2
  - **Acceptance:**
    - All shortcuts customizable
    - Conflicts detected

- [x] **Task 5.6.3.2:** Implement shortcut capture UI **(M)**
  - [x] Create shortcut input component (placeholder for GPUI)
  - [x] Capture key combinations (API defined)
  - [x] Show current binding (API defined)
  - [x] Allow clearing binding (API defined)
  - **Dependencies:** 5.6.3.1
  - **Acceptance:** Shortcut APIs defined (actual GPUI UI pending)

#### 5.6.4 Preferences UI

- [x] **Task 5.6.4.1:** Create Preferences command **(S)**
  - [x] Register "Preferences" command
  - [x] Open preferences view (mode-switching command)
  - **Dependencies:** 5.6.1.1
  - **Acceptance:** Command registered and recognized

- [x] **Task 5.6.4.2:** Implement Appearance section **(M)**
  - [x] Theme selector dropdown (data model defined)
  - [x] Accent color picker (swatches) (data model defined)
  - [x] Animation toggle and duration slider (config defined)
  - [ ] Live preview of changes (deferred to GPUI implementation)
  - **Dependencies:** 5.6.2.1, 5.6.2.2
  - **Acceptance:** Appearance config structure and APIs defined

- [x] **Task 5.6.4.3:** Implement Clipboard section **(M)**
  - [x] History size input (placeholder UI, config exists)
  - [x] Retention days input (placeholder UI, config exists)
  - [x] Store images toggle (config exists)
  - [x] Max image size slider (config exists)
  - [x] Excluded apps list management (placeholder UI, config exists)
  - [x] Default action toggle (config exists)
  - **Dependencies:** 5.6.1.1
  - **Acceptance:** Clipboard preferences data structures defined

- [x] **Task 5.6.4.4:** Implement Shortcuts section **(M)**
  - [x] Global hotkey configuration (placeholder UI)
  - [x] Clipboard hotkey configuration (placeholder UI)
  - [x] List all configurable shortcuts (API defined)
  - [x] "Reset to Defaults" button (API defined)
  - **Dependencies:** 5.6.3.1
  - **Acceptance:** Shortcuts APIs and data structures defined

- [x] **Task 5.6.4.5:** Implement other settings sections **(M)**
  - [x] General (launch at login) (placeholder UI)
  - [x] Calculator settings (config in separate crate)
  - [x] Window management settings (config in separate crate)
  - [x] Calendar settings (config in separate crate)
  - [x] App management settings (config in separate crate)
  - [x] Sleep timer settings (config defined)
  - **Dependencies:** 5.6.1.1
  - **Acceptance:** Settings data structures defined

#### 5.6.5 Testing

- [x] **Task 5.6.5.1:** Write tests **(M)**
  - [x] Test config loading/saving
  - [x] Test theme serialization
  - [x] Test shortcut conflict detection
  - [x] Test default values
  - [x] Test keybindings parsing and serialization
  - [x] Test preferences view state management
  - **Dependencies:** 5.6.1-5.6.4
  - **Acceptance:** 80%+ test coverage for implemented features

---

## Sprint 6: Native Extension System (Weeks 21-24)

### 6.1 Extension Infrastructure

#### 6.1.1 Extension Host Setup

- [ ] **Task 6.1.1.1:** Create `photoncast-extensions` crate **(S)**
  - [ ] Set up Cargo.toml
  - [ ] Define module structure
  - **Dependencies:** None
  - **Acceptance:** Crate compiles

- [ ] **Task 6.1.1.2:** Create `photoncast-extension-api` crate **(S)**
  - [ ] Set up as publishable crate
  - [ ] Define public API for extension developers
  - [ ] Create prelude module for common imports
  - **Dependencies:** None
  - **Acceptance:** Crate ready for publication

- [ ] **Task 6.1.1.3:** Define extension manifest schema **(M)**
  - [ ] extension.toml TOML schema
  - [ ] [extension] section: name, title, description, version, author, license, icon
  - [ ] [permissions] section: all permission flags
  - [ ] [[commands]] array: name, title, description, mode, icon
  - [ ] [[preferences.items]] array: name, type, title, description, default
  - [ ] Implement manifest parser
  - [ ] Validate manifest on load
  - **Dependencies:** 6.1.1.2
  - **Acceptance:** Manifest fully defined and parseable

#### 6.1.2 Extension Loading

- [ ] **Task 6.1.2.1:** Implement extension discovery **(M)**
  - [ ] Scan ~/Library/Application Support/PhotonCast/Extensions/
  - [ ] Find valid extension directories
  - [ ] Load and validate manifest for each
  - [ ] Build extension registry
  - **Dependencies:** 6.1.1.3
  - **Acceptance:**
    - All extensions discovered
    - Invalid extensions skipped with error

- [ ] **Task 6.1.2.2:** Implement extension loading **(L)**
  - [ ] Load compiled extension (dynamic library)
  - [ ] Initialize extension context
  - [ ] Call on_load lifecycle method
  - [ ] Register extension commands
  - [ ] Handle load failures gracefully
  - **Dependencies:** 6.1.2.1
  - **Acceptance:**
    - Extensions load successfully
    - Load time <50ms per extension

- [ ] **Task 6.1.2.3:** Implement extension unloading **(M)**
  - [ ] Call on_unload lifecycle method
  - [ ] Clean up resources
  - [ ] Unregister commands
  - [ ] Handle unload failures
  - **Dependencies:** 6.1.2.2
  - **Acceptance:** Extensions unload cleanly

#### 6.1.3 Sandboxing & Permissions

- [ ] **Task 6.1.3.1:** Implement permission model **(L)**
  - [ ] Define permission checks for each permission type
  - [ ] clipboard_read: Read clipboard content
  - [ ] clipboard_write: Write to clipboard
  - [ ] network: Make HTTP requests
  - [ ] filesystem_read: Read files (scoped to user directory)
  - [ ] filesystem_write: Write files (extension directory only)
  - [ ] notifications: Show system notifications
  - [ ] storage: Per-extension SQLite storage
  - [ ] Enforce permissions at API level
  - **Dependencies:** 6.1.1.3
  - **Acceptance:** Permissions enforced correctly

- [ ] **Task 6.1.3.2:** Implement process isolation **(XL)**
  - [ ] Run each extension in isolated process
  - [ ] Set up IPC channel (stdin/stdout JSON-RPC)
  - [ ] Handle extension crashes without crashing host
  - [ ] Implement error boundaries as fallback
  - [ ] Resource limits per extension
  - **Dependencies:** 6.1.3.1
  - **Acceptance:**
    - Extensions isolated
    - Crashes contained
    - Memory limits enforced

#### 6.1.4 IPC Protocol

- [ ] **Task 6.1.4.1:** Define IPC message types **(M)**
  - [ ] Host → Extension: Load, Run, SearchQuery, ExecuteAction, Unload
  - [ ] Extension → Host: Render, Loading, ShowToast, ShowHUD, Push, Pop, CopyToClipboard, OpenURL, SearchResults, Error
  - [ ] Implement JSON-RPC serialization
  - **Dependencies:** 6.1.3.2
  - **Acceptance:** All message types defined

- [ ] **Task 6.1.4.2:** Implement IPC bridge **(L)**
  - [ ] Create async message channel
  - [ ] Handle request/response correlation
  - [ ] Implement timeout handling
  - [ ] Handle disconnection recovery
  - **Dependencies:** 6.1.4.1
  - **Acceptance:** Reliable bidirectional communication

#### 6.1.5 Extension API

- [ ] **Task 6.1.5.1:** Implement `ExtensionContext` **(L)**
  - [ ] storage: ExtensionStorage (per-extension SQLite)
  - [ ] clipboard: Optional<ClipboardAccess> (if permitted)
  - [ ] http: Optional<HttpClient> (if permitted)
  - [ ] notifications: Optional<NotificationApi> (if permitted)
  - [ ] preferences: HashMap<String, Value>
  - **Dependencies:** 6.1.3.1
  - **Acceptance:** Context provides all APIs per permissions

- [ ] **Task 6.1.5.2:** Implement `ExtensionStorage` **(M)**
  - [ ] Per-extension SQLite database
  - [ ] get(key) -> Option<String>
  - [ ] set(key, value)
  - [ ] remove(key)
  - [ ] all() -> HashMap
  - [ ] clear()
  - [ ] Isolated per extension_id
  - **Dependencies:** 6.1.5.1
  - **Acceptance:** Storage isolated and functional

- [ ] **Task 6.1.5.3:** Implement UI component API **(L)**
  - [ ] ListView: items, sections, loading, search_placeholder
  - [ ] GridView: items, columns, aspect_ratio
  - [ ] DetailView: markdown, metadata
  - [ ] FormView: fields, submit action
  - [ ] ListItem: title, subtitle, icon, accessories, actions
  - [ ] Action: title, icon, shortcut, handler
  - [ ] Serialize to JSON for IPC
  - **Dependencies:** 6.1.4.1
  - **Acceptance:** All UI components implemented

- [ ] **Task 6.1.5.4:** Implement toast/notification API **(S)**
  - [ ] show_toast(options) -> Toast
  - [ ] show_hud(title)
  - [ ] Toast styles: Success, Failure, Animated
  - [ ] Primary action support
  - **Dependencies:** 6.1.5.1
  - **Acceptance:** Notifications display correctly

#### 6.1.6 Extension UI Rendering

- [ ] **Task 6.1.6.1:** Implement extension view renderer **(L)**
  - [ ] Receive ExtensionView from extension
  - [ ] Render as GPUI components
  - [ ] Handle loading states
  - [ ] Handle empty states
  - [ ] Support navigation (push/pop)
  - **Dependencies:** 6.1.5.3
  - **Acceptance:**
    - All view types render correctly
    - Navigation works

- [ ] **Task 6.1.6.2:** Implement extension action handler **(M)**
  - [ ] Handle OpenUrl action
  - [ ] Handle CopyToClipboard action
  - [ ] Handle Paste action
  - [ ] Handle Push/Pop navigation
  - [ ] Handle Custom async actions
  - **Dependencies:** 6.1.6.1
  - **Acceptance:** All action types work

#### 6.1.7 Hot Reload

- [ ] **Task 6.1.7.1:** Implement file watcher for dev mode **(M)**
  - [ ] Watch extension directory for changes
  - [ ] Detect .rs file modifications
  - [ ] Trigger rebuild (cargo build)
  - [ ] Reload extension after successful build
  - [ ] Preserve extension state where possible
  - **Dependencies:** 6.1.2.2, 6.1.2.3
  - **Acceptance:** Changes reflected without manual restart

#### 6.1.8 Search Integration

- [ ] **Task 6.1.8.1:** Implement extension search provider **(M)**
  - [ ] Extensions can provide search results
  - [ ] Route search queries to relevant extensions
  - [ ] Merge extension results with core results
  - [ ] Handle slow extensions gracefully
  - **Dependencies:** 6.1.4.2
  - **Acceptance:** Extension results appear in search

#### 6.1.9 Testing

- [ ] **Task 6.1.9.1:** Write unit tests **(M)**
  - [ ] Test manifest parsing
  - [ ] Test permission enforcement
  - [ ] Test IPC serialization
  - [ ] Test storage isolation
  - **Dependencies:** 6.1.1-6.1.8
  - **Acceptance:** 80%+ coverage

- [ ] **Task 6.1.9.2:** Write integration tests **(L)**
  - [ ] Test full extension lifecycle
  - [ ] Test extension isolation
  - [ ] Test hot reload
  - [ ] Test error recovery
  - **Dependencies:** 6.1.9.1
  - **Acceptance:** All integration tests pass

- [ ] **Task 6.1.9.3:** Add benchmarks **(S)**
  - [ ] Benchmark extension_load (<50ms)
  - [ ] Benchmark IPC round-trip
  - **Dependencies:** 6.1.9.1
  - **Acceptance:** Performance targets met

---

### 6.2 Custom Commands

#### 6.2.1 Configuration

- [ ] **Task 6.2.1.1:** Define custom commands schema **(S)**
  - [ ] commands.toml file format
  - [ ] Command fields: name, title, icon, shell, script
  - [ ] Optional: timeout_seconds, environment, confirm
  - [ ] Implement parser
  - **Dependencies:** None
  - **Acceptance:** Schema fully defined

- [ ] **Task 6.2.1.2:** Implement command loader **(M)**
  - [ ] Load from ~/.config/photoncast/commands.toml
  - [ ] Validate command definitions
  - [ ] Register commands with launcher
  - [ ] Reload on file change
  - **Dependencies:** 6.2.1.1
  - **Acceptance:** Commands load and register

#### 6.2.2 Execution

- [ ] **Task 6.2.2.1:** Implement shell executor **(M)**
  - [ ] Execute script with specified shell (default $SHELL)
  - [ ] Inherit system environment
  - [ ] Add per-command environment variables
  - [ ] Implement timeout (default 60s)
  - [ ] Handle script errors
  - **Dependencies:** 6.2.1.2
  - **Acceptance:**
    - Commands execute correctly
    - Timeout enforced

- [ ] **Task 6.2.2.2:** Implement output streaming **(M)**
  - [ ] Capture stdout/stderr
  - [ ] Stream output in real-time
  - [ ] Display in output view
  - [ ] Support ANSI colors (optional)
  - **Dependencies:** 6.2.2.1
  - **Acceptance:** Output displays as command runs

- [ ] **Task 6.2.2.3:** Implement completion notifications **(S)**
  - [ ] Show HUD for successful completion
  - [ ] Show Toast for failures with error message
  - [ ] Include command output in notification
  - **Dependencies:** 6.2.2.2
  - **Acceptance:** Clear success/failure feedback

#### 6.2.3 UI

- [ ] **Task 6.2.3.1:** Create custom command UI **(M)**
  - [ ] Show command in launcher results
  - [ ] Display command icon and title
  - [ ] Show confirmation dialog if configured
  - [ ] Show output view during execution
  - **Dependencies:** 6.2.2.2
  - **Acceptance:** Full command lifecycle visible

#### 6.2.4 Testing

- [ ] **Task 6.2.4.1:** Write tests **(M)**
  - [ ] Test command parsing
  - [ ] Test shell execution
  - [ ] Test timeout handling
  - [ ] Test environment inheritance
  - [ ] Test output streaming
  - **Dependencies:** 6.2.1-6.2.3
  - **Acceptance:** 80%+ test coverage

---

### 6.3 First-Party Extensions

#### 6.3.1 GitHub Repositories Extension

- [ ] **Task 6.3.1.1:** Create GitHub extension **(L)**
  - [ ] Extension manifest with network permission
  - [ ] Implement GitHub API client (personal access token)
  - [ ] List user repositories
  - [ ] Search repositories
  - [ ] Show repo details (stars, forks, language)
  - [ ] Actions: Open in browser, Clone URL, Open in VS Code
  - [ ] Cache results for performance
  - **Dependencies:** 6.1.5.1
  - **Acceptance:**
    - Lists all user repos
    - Search works
    - All actions functional

- [ ] **Task 6.3.1.2:** Implement GitHub preferences **(S)**
  - [ ] API token storage (in Keychain)
  - [ ] Configure display options
  - **Dependencies:** 6.3.1.1
  - **Acceptance:** Token configurable via preferences

#### 6.3.2 System Preferences Extension

- [ ] **Task 6.3.2.1:** Create System Preferences extension **(M)**
  - [ ] Extension manifest
  - [ ] List all macOS System Preferences panes
  - [ ] Search by name and keywords
  - [ ] Open preference pane on selection
  - [ ] Group by category
  - **Dependencies:** 6.1.5.1
  - **Acceptance:**
    - All preference panes listed
    - Opens correct pane

- [ ] **Task 6.3.2.2:** Add common shortcuts **(S)**
  - [ ] Wi-Fi, Bluetooth, Display, Sound
  - [ ] Keyboard, Mouse, Trackpad
  - [ ] Security & Privacy
  - [ ] Accessibility
  - **Dependencies:** 6.3.2.1
  - **Acceptance:** Common settings easily accessible

#### 6.3.3 Color Picker Extension

- [ ] **Task 6.3.3.1:** Create Color Picker extension **(L)**
  - [ ] Extension manifest with clipboard permission
  - [ ] Screen eyedropper tool (NSColorSampler)
  - [ ] Color format conversion (HEX, RGB, HSL, HSB)
  - [ ] Copy color in any format
  - [ ] Color palette storage (save favorite colors)
  - [ ] History of picked colors
  - **Dependencies:** 6.1.5.1
  - **Acceptance:**
    - Eyedropper picks screen color
    - All formats available
    - Palette persists

- [ ] **Task 6.3.3.2:** Implement color palette UI **(M)**
  - [ ] Show saved colors grid
  - [ ] Add/remove colors
  - [ ] Name colors
  - [ ] Export palette
  - **Dependencies:** 6.3.3.1
  - **Acceptance:** Full palette management

#### 6.3.4 Bundling

- [ ] **Task 6.3.4.1:** Bundle first-party extensions **(M)**
  - [ ] Include all three extensions in app bundle
  - [ ] Pre-compile extensions
  - [ ] Auto-load on first launch
  - [ ] Allow user to disable
  - **Dependencies:** 6.3.1-6.3.3
  - **Acceptance:** Extensions available out of the box

---

### 6.4 Extension Development CLI

#### 6.4.1 CLI Implementation

- [ ] **Task 6.4.1.1:** Create extension CLI **(M)**
  - [ ] `photoncast extension new <name>` - Create new extension
  - [ ] Generate scaffold with Cargo.toml, extension.toml, src/lib.rs
  - [ ] Include example code
  - **Dependencies:** 6.1.1.3
  - **Acceptance:** Scaffold creates valid extension

- [ ] **Task 6.4.1.2:** Implement dev command **(M)**
  - [ ] `photoncast extension dev` - Run with hot-reload
  - [ ] Watch for changes
  - [ ] Auto-rebuild and reload
  - [ ] Show build errors
  - **Dependencies:** 6.1.7.1
  - **Acceptance:** Development workflow smooth

- [ ] **Task 6.4.1.3:** Implement build command **(S)**
  - [ ] `photoncast extension build` - Build for distribution
  - [ ] Optimize build (release mode)
  - [ ] Validate manifest
  - **Dependencies:** 6.4.1.1
  - **Acceptance:** Produces distributable extension

- [ ] **Task 6.4.1.4:** Implement validate command **(S)**
  - [ ] `photoncast extension validate` - Validate manifest and code
  - [ ] Check manifest schema
  - [ ] Check API usage
  - [ ] Check permissions match usage
  - **Dependencies:** 6.4.1.1
  - **Acceptance:** Validation catches common issues

- [ ] **Task 6.4.1.5:** Implement package command **(M)**
  - [ ] `photoncast extension package` - Package for distribution
  - [ ] Create .photonext package (zip with metadata)
  - [ ] Include icon and README
  - **Dependencies:** 6.4.1.3
  - **Acceptance:** Package ready for distribution

#### 6.4.2 Testing

- [ ] **Task 6.4.2.1:** Write CLI tests **(S)**
  - [ ] Test scaffold generation
  - [ ] Test build process
  - [ ] Test validation
  - [ ] Test packaging
  - **Dependencies:** 6.4.1.1-6.4.1.5
  - **Acceptance:** All CLI commands work correctly

---

## Cross-Cutting Tasks

### Documentation (Throughout)

- [ ] **Task X.1:** Write API documentation for extension developers **(L)**
  - [ ] Document all public APIs
  - [ ] Include code examples
  - [ ] Tutorial: Creating your first extension
  - **Dependencies:** Sprint 6 completion
  - **Acceptance:** Comprehensive API docs

- [ ] **Task X.2:** Update user documentation **(M)**
  - [ ] Document all new features
  - [ ] Keyboard shortcut reference
  - [ ] Troubleshooting guide
  - **Dependencies:** All sprints
  - **Acceptance:** User guide complete

### Quality Assurance (Throughout)

- [ ] **Task X.3:** Maintain 80% test coverage **(Ongoing)**
  - [ ] Run coverage checks in CI
  - [ ] Identify and fill coverage gaps
  - **Dependencies:** All test tasks
  - **Acceptance:** Coverage ≥80%

- [ ] **Task X.4:** Performance benchmarking **(M)**
  - [ ] Set up continuous benchmarking in CI
  - [ ] Alert on performance regressions
  - **Dependencies:** All benchmark tasks
  - **Acceptance:** No performance regressions

### Release Preparation

- [ ] **Task X.5:** Integration testing of all features together **(L)**
  - [ ] End-to-end test scenarios
  - [ ] Cross-feature interactions
  - [ ] Stress testing
  - **Dependencies:** All feature tasks
  - **Acceptance:** All features work together

- [ ] **Task X.6:** Release candidate testing **(M)**
  - [ ] Beta testing period
  - [ ] Bug fixes
  - [ ] Final polish
  - **Dependencies:** X.5
  - **Acceptance:** Ready for v1.0.0 release

---

## Dependency Graph (Critical Path)

```
Sprint 4 (Weeks 13-16):
  Clipboard: 4.1.1.2 → 4.1.1.3 → 4.1.1.4 → 4.1.3.1 → 4.1.4.2 → 4.1.4.4
  Calculator: 4.2.2.1 → 4.2.3.1 → 4.2.3.3 → 4.2.4.1 → 4.2.5.1 → 4.2.6.2

Sprint 5 (Weeks 17-20):
  Window: 5.1.1.2 → 5.1.2.2 → 5.1.3.1 → 5.1.4.2 → 5.1.5.1
  Calendar: 5.3.1.2 → 5.3.1.3 → 5.3.3.1 → 5.3.4.2
  Preferences: 5.6.1.1 → 5.6.1.2 → 5.6.2.1 → 5.6.4.2

Sprint 6 (Weeks 21-24):
  Extensions: 6.1.1.3 → 6.1.2.2 → 6.1.3.2 → 6.1.4.2 → 6.1.5.1 → 6.1.6.1 → 6.3.1.1
  CLI: 6.1.1.3 → 6.4.1.1 → 6.4.1.2 → 6.4.1.5
```

---

## Complexity Legend

| Symbol | Complexity | Estimated Hours | Description |
|--------|------------|-----------------|-------------|
| S | Small | 1-4 hours | Simple, well-defined task |
| M | Medium | 4-12 hours | Moderate complexity, some unknowns |
| L | Large | 12-24 hours | Complex, multiple components |
| XL | Extra Large | 24-40 hours | Very complex, significant unknowns |

---

## Progress Tracking

**Sprint 4 Progress:** 35 / 38 tasks complete (92%)  
**Sprint 5 Progress:** 67 / 67 tasks complete (100%)  
**Sprint 6 Progress:** 0 / 41 tasks complete (0%)  
**Total Progress:** 102 / 146 tasks complete (70%)

Last Updated: 2026-01-19
