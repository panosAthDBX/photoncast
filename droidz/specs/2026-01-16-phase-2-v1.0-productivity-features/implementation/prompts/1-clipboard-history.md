# Implementation: Clipboard History (4.1)

## Task Assignment

You are implementing the **Clipboard History** feature for PhotonCast - a Rust-based macOS launcher using GPUI.

### Task Group: 4.1 Clipboard History (42 tasks)

#### 4.1.1 Infrastructure & Storage Setup

- [ ] **Task 4.1.1.1:** Create `photoncast-clipboard` crate structure **(S)**
  - [ ] Set up Cargo.toml with dependencies (rusqlite, aes-gcm, argon2)
  - [ ] Create lib.rs with module structure
  - [ ] Define public API surface
  - **Dependencies:** None
  - **Acceptance:** Crate compiles, basic structure in place

- [ ] **Task 4.1.1.2:** Implement encryption manager **(M)**
  - [ ] Create `EncryptionManager` struct
  - [ ] Implement machine-derived key derivation using argon2
  - [ ] Implement AES-256-GCM encryption
  - [ ] Implement AES-256-GCM decryption
  - [ ] Add nonce generation and handling
  - [ ] Write unit tests for encrypt/decrypt roundtrip
  - **Dependencies:** None
  - **Acceptance:** 
    - Encryption/decryption works correctly
    - Key is deterministic per machine
    - Tests pass with 100% coverage for encryption module

- [ ] **Task 4.1.1.3:** Design and implement SQLite schema **(M)**
  - [ ] Create `clipboard_items` table with all content type fields
  - [ ] Create FTS5 virtual table for full-text search (`clipboard_fts`)
  - [ ] Add indexes for created_at, pinned status
  - [ ] Implement schema migration system
  - [ ] Add database initialization code
  - **Dependencies:** 4.1.1.2
  - **Acceptance:**
    - Schema creates successfully
    - All content types storable
    - FTS5 search functional

- [ ] **Task 4.1.1.4:** Implement `ClipboardStorage` struct **(L)**
  - [ ] Create async-safe storage wrapper
  - [ ] Implement `store()` method with encryption
  - [ ] Implement `load_recent(limit)` method with decryption
  - [ ] Implement `load_pinned()` method
  - [ ] Implement `search(query)` using FTS5
  - [ ] Implement `pin(id)` / `unpin(id)` methods
  - [ ] Implement `delete(id)` method
  - [ ] Implement `clear_all()` method
  - [ ] Add retention policy enforcement (30-day default)
  - [ ] Write integration tests
  - **Dependencies:** 4.1.1.2, 4.1.1.3
  - **Acceptance:**
    - All CRUD operations work
    - Encryption transparent to callers
    - Retention policy enforced automatically
    - 80%+ test coverage

#### 4.1.2 Data Models

- [ ] **Task 4.1.2.1:** Define `ClipboardContentType` enum **(M)**
  - [ ] Implement `Text` variant with content and preview
  - [ ] Implement `RichText` variant with plain/html/rtf
  - [ ] Implement `Image` variant with path, thumbnail, dimensions
  - [ ] Implement `File` variant with paths and icons
  - [ ] Implement `Link` variant with URL, title, favicon
  - [ ] Implement `Color` variant with hex, rgb, display name
  - [ ] Derive Serialize/Deserialize
  - [ ] Write serialization tests
  - **Dependencies:** None
  - **Acceptance:** All content types serialize/deserialize correctly

- [ ] **Task 4.1.2.2:** Define `ClipboardItem` struct **(S)**
  - [ ] Add id, content_type fields
  - [ ] Add source_app, source_bundle_id fields
  - [ ] Add size_bytes, is_pinned fields
  - [ ] Add created_at, accessed_at timestamps
  - [ ] Implement Display trait for previews
  - **Dependencies:** 4.1.2.1
  - **Acceptance:** Struct fully defined with all metadata

#### 4.1.3 Clipboard Monitoring

- [ ] **Task 4.1.3.1:** Implement `ClipboardMonitor` **(L)**
  - [ ] Create NSPasteboard wrapper using objc2
  - [ ] Implement polling loop (250ms interval)
  - [ ] Detect clipboard changes via changeCount
  - [ ] Parse pasteboard contents by type (UTI detection)
  - [ ] Extract text content (NSStringPboardType)
  - [ ] Extract rich text (RTF, HTML)
  - [ ] Extract images (PNG, TIFF, JPEG)
  - [ ] Extract file references (NSFilenamesPboardType)
  - [ ] Detect URLs with title extraction
  - [ ] Detect color values (hex/rgb patterns)
  - [ ] Respect NSPasteboardTransient flag
  - [ ] Write unit tests with mock pasteboard
  - **Dependencies:** 4.1.2.1
  - **Acceptance:**
    - Detects all copy events
    - Correctly identifies content types
    - Ignores transient items
    - <5ms per check

- [ ] **Task 4.1.3.2:** Implement app exclusion filter **(S)**
  - [ ] Load excluded apps from config
  - [ ] Detect source app bundle ID
  - [ ] Filter password managers by default:
    - com.1password.1password
    - com.agilebits.onepassword7
    - com.bitwarden.desktop
    - com.lastpass.LastPass
    - com.apple.keychainaccess
    - com.dashlane.Dashlane
  - [ ] Allow user-configurable exclusions
  - [ ] Write tests for exclusion logic
  - **Dependencies:** 4.1.3.1
  - **Acceptance:** Excluded apps never stored

- [ ] **Task 4.1.3.3:** Implement image handling **(M)**
  - [ ] Check image size against max (10MB default)
  - [ ] Store full image to app data directory
  - [ ] Generate thumbnail (200x200 max)
  - [ ] Store thumbnail for fast preview
  - [ ] Extract dimensions metadata
  - [ ] Clean up orphaned images on delete
  - **Dependencies:** 4.1.3.1
  - **Acceptance:**
    - Images under limit stored correctly
    - Thumbnails generated for all images
    - Large images rejected gracefully

- [ ] **Task 4.1.3.4:** Implement URL metadata fetching **(M)**
  - [ ] Detect URLs in clipboard content
  - [ ] Fetch page title in background (reqwest)
  - [ ] Fetch and cache favicon
  - [ ] Handle fetch failures gracefully
  - [ ] Implement caching to avoid refetching
  - [ ] Add timeout (5 seconds)
  - **Dependencies:** 4.1.3.1
  - **Acceptance:**
    - URLs display with title and favicon
    - Failures don't block clipboard storage
    - Cache prevents duplicate fetches

#### 4.1.4 UI Components

- [ ] **Task 4.1.4.1:** Create clipboard history command **(M)**
  - [ ] Register "Clipboard History" command with launcher
  - [ ] Set default hotkey: Cmd+Shift+V
  - [ ] Create dedicated view for clipboard UI
  - [ ] Implement icon and description
  - **Dependencies:** 4.1.1.4
  - **Acceptance:** Command appears in launcher, opens clipboard view

- [ ] **Task 4.1.4.2:** Implement clipboard list view **(L)**
  - [ ] Create GPUI view component
  - [ ] Display "Pinned" section at top
  - [ ] Display "Recent" section below
  - [ ] Show content type icon per item
  - [ ] Show preview text (100 chars max)
  - [ ] Show timestamp (relative: "Just now", "5 min ago")
  - [ ] Show color swatches for color items
  - [ ] Show thumbnails for image items
  - [ ] Show favicon + title for URL items
  - [ ] Implement keyboard navigation (↑/↓)
  - [ ] Implement selection highlighting
  - **Dependencies:** 4.1.4.1
  - **Acceptance:**
    - All content types display correctly
    - Smooth 60fps scrolling
    - Clear visual hierarchy

- [ ] **Task 4.1.4.3:** Implement clipboard search **(M)**
  - [ ] Add search input at top
  - [ ] Filter results using FTS5 in real-time
  - [ ] Highlight matching text in results
  - [ ] Show "No results" state
  - [ ] Debounce search input (100ms)
  - **Dependencies:** 4.1.4.2
  - **Acceptance:**
    - Search is instant (<50ms)
    - Results update as user types
    - Matches highlighted visually

- [ ] **Task 4.1.4.4:** Implement clipboard actions **(L)**
  - [ ] **Paste (Enter):** Paste directly to frontmost app
  - [ ] **Copy (Cmd+C):** Copy to clipboard without pasting
  - [ ] **Paste as Plain Text (Cmd+Shift+V):** Strip formatting
  - [ ] **Paste and Don't Save (Cmd+Opt+V):** One-time paste
  - [ ] **Pin/Unpin (Cmd+P):** Toggle pinned status
  - [ ] **Delete (Cmd+Backspace):** Remove from history
  - [ ] **Clear All (Cmd+Shift+Backspace):** With confirmation dialog
  - [ ] Show action panel with shortcuts
  - [ ] Make default action configurable (paste vs copy)
  - **Dependencies:** 4.1.4.2
  - **Acceptance:**
    - All actions work correctly
    - Keyboard shortcuts functional
    - Confirmation required for destructive actions

#### 4.1.5 Testing

- [ ] **Task 4.1.5.1:** Write unit tests **(M)**
  - [ ] Test encryption roundtrip
  - [ ] Test all content type parsing
  - [ ] Test exclusion filter
  - [ ] Test image size validation
  - [ ] Test retention policy
  - [ ] Test FTS5 search
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

- [ ] **Task 4.1.5.3:** Add benchmarks **(S)**
  - [ ] Benchmark clipboard_load_1000 (<100ms)
  - [ ] Benchmark clipboard_search (<50ms)
  - [ ] Benchmark encryption/decryption
  - **Dependencies:** 4.1.5.1
  - **Acceptance:** Performance targets met

---

## Context Files

Read these for requirements and patterns:
- **Spec:** `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md`
- **Requirements:** `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/requirements-answers.md`

## Standards to Follow

Read and follow ALL standards from `droidz/standards/`:
- `global/tech-stack.md` - Rust, GPUI, Tokio stack
- `global/coding-style.md` - Rust conventions (type safety, iterators, etc.)
- `global/error-handling.md` - Use thiserror + anyhow patterns
- `global/crate-first.md` - Always search for crates before implementing
- `backend/platform.md` - macOS integration patterns (objc2, NSPasteboard)
- `frontend/components.md` - GPUI component patterns
- `testing/test-writing.md` - Test patterns (80% coverage required)

## Key Requirements

From the requirements answers:
- **Encryption:** AES-256-GCM with machine-derived key via argon2
- **Storage:** Encrypted SQLite with FTS5 search
- **Retention:** 30 days default, 1000 items max (configurable)
- **Content Types:** Text, RichText, Image (10MB max), File, Link (with favicon fetch), Color (with swatch)
- **Excluded Apps:** 1Password, Bitwarden, LastPass, Keychain, Dashlane by default
- **Transient Items:** Never store (respect NSPasteboardTransient)
- **Hotkey:** Cmd+Shift+V
- **Default Action:** Configurable (paste vs copy), default to paste
- **Pinned Items:** Separate section, don't count against limit

## Instructions

1. Read and analyze the spec.md for detailed requirements
2. Study existing codebase patterns in `crates/` directory
3. Create `crates/photoncast-clipboard/` with proper structure
4. Implement features in dependency order (infrastructure → models → monitoring → UI)
5. Write tests alongside implementation (aim for 80%+ coverage)
6. Run `cargo test` and `cargo clippy` to verify
7. Mark completed tasks with `[x]` in `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/tasks.md`

## Crates to Evaluate/Use

- `rusqlite` - SQLite with FTS5 support
- `aes-gcm` - AES-256-GCM encryption
- `argon2` - Key derivation
- `objc2` + `objc2-app-kit` - NSPasteboard access
- `reqwest` - Async HTTP for favicon/title fetching
- `image` - Thumbnail generation
- `chrono` - Timestamps
