# File Search - Raycast Parity Tasks

**Status:** Draft  
**Estimated Duration:** 12 weeks  
**Dependencies:** Phase 1 MVP (Complete)

---

## Task Dependency Graph

```
Phase 0: UI Foundation (CRITICAL - Must be first)
├── 0.1 Split-View Layout ───────────────────────┐
├── 0.2 List Item Component                      │
├── 0.3 Detail Panel Component                   │
├── 0.4 Search Bar with Dropdown                 │
└── 0.5 Sections & Empty State                   │
         │                                       │
         ▼                                       │
Phase 1: Core Enhancements                       │
├── 1.1 Missing File Actions ────────────────────┤
├── 1.2 File Type Filtering ─────────────────────┼──► Phase 2: Query Syntax
├── 1.3 Quick Look Enhancement                   │    ├── 2.1 Natural Language
└── 1.4 External Drive Support                   │    ├── 2.2 Folder Priority
                                                 │    └── 2.3 Parent Folder Search
                                                 │
Phase 3: Browsing Mode ◄─────────────────────────┘
├── 3.1 Path Detection
├── 3.2 Tab Navigation
├── 3.3 Directory Filtering
└── 3.4 Environment Variables
         │
         ▼
Phase 4: Custom Index
├── 4.1 Tokenization Engine
├── 4.2 Background Indexing
├── 4.3 File System Watcher
└── 4.4 SQLite Storage
         │
         ▼
Phase 5: Ignore Patterns
├── 5.1 Pattern Parser
├── 5.2 Hierarchical Matching
└── 5.3 Exclude Action
         │
         ▼
Phase 6: Preferences & Polish
├── 6.1 Preferences UI
├── 6.2 Search Scope Management
└── 6.3 Testing & Optimization
```

---

## Phase 0: UI Foundation (Week 1-2) ⭐ CRITICAL

> This phase implements the core Raycast-style UI layout that all other features depend on.

### 0.1 Split-View Layout (Priority: Critical)

- [ ] **0.1.1 Create FileSearchView component** (3h)
  - Two-panel horizontal split layout
  - Left: scrollable list (60% width)
  - Right: detail panel (40% width)
  - Resizable divider (optional)
  - Dependencies: None

- [ ] **0.1.2 Implement panel state management** (2h)
  - Track selected file
  - Update detail panel on selection change
  - Handle keyboard navigation between panels
  - Dependencies: 0.1.1

- [ ] **0.1.3 Add responsive behavior** (1h)
  - Minimum widths for each panel
  - Handle window resize
  - Dependencies: 0.1.1

### 0.2 List Item Component (Priority: Critical)

- [ ] **0.2.1 Create FileListItem component** (3h)
  - Icon (left) - file type or actual file icon
  - Title (file name)
  - Subtitle (parent folder path, e.g., `~/Documents`)
  - Accessories (right) - relative date
  - Dependencies: 0.1.1

- [ ] **0.2.2 Implement file type icons** (2h)
  - Map extensions to icons (📁📄📕📊🖼️🎬🎵💻⚙️)
  - Use NSWorkspace.icon(forFile:) when available
  - Fallback to generic icons
  - Dependencies: 0.2.1

- [ ] **0.2.3 Implement date formatting** (1h)
  - Relative dates: "Just now", "5m", "3h", "Yesterday", "3d", "Jan 15"
  - Handle edge cases (future dates, very old files)
  - Dependencies: 0.2.1

- [ ] **0.2.4 Implement size formatting** (1h)
  - Human-readable: "500 bytes", "2.4 KB", "1.5 MB", "3.2 GB"
  - Dependencies: 0.2.1

- [ ] **0.2.5 Add selection highlighting** (1h)
  - Visual indicator for selected item (▸ or background color)
  - Keyboard focus ring
  - Dependencies: 0.2.1

### 0.3 Detail Panel Component (Priority: Critical)

- [ ] **0.3.1 Create FileDetailPanel component** (3h)
  - Preview area (top 60%)
  - Metadata section (bottom 40%)
  - Scrollable if content overflows
  - Dependencies: 0.1.1

- [ ] **0.3.2 Implement preview area** (4h)
  - Quick Look preview for supported files
  - Thumbnail for images
  - File type icon for unsupported files
  - Loading state while preview loads
  - Dependencies: 0.3.1

- [ ] **0.3.3 Implement metadata section** (3h)
  - Label-value pairs in two columns
  - Fields: Name, Kind, Size, Created, Modified, Where
  - For folders: add "Items" field
  - Dependencies: 0.3.1

- [ ] **0.3.4 Implement Kind detection** (2h)
  - Use UTType to get localized description
  - "PDF Document", "PNG Image", "Folder", etc.
  - Dependencies: 0.3.3

### 0.4 Search Bar with Dropdown (Priority: Critical)

- [ ] **0.4.1 Create FileSearchBar component** (2h)
  - Search icon (🔍)
  - Text input with placeholder "Search files by name..."
  - Dropdown accessory on right
  - Dependencies: 0.1.1

- [ ] **0.4.2 Implement dropdown filter** (3h)
  - Trigger with click or `⌘P`
  - Options: All Files, Documents, Images, Videos, Audio, Archives, Code, Folders
  - Radio button selection
  - Persist selection (`storeValue: true`)
  - Dependencies: 0.4.1

- [ ] **0.4.3 Add filter badge** (1h)
  - Show active filter name in dropdown button
  - "All Files ▾" → "Images ▾"
  - Dependencies: 0.4.2

### 0.5 Sections & Empty State (Priority: High)

- [ ] **0.5.1 Implement List.Section component** (2h)
  - Section header with title
  - Collapsible (optional)
  - Dependencies: 0.1.1

- [ ] **0.5.2 Add "Recent Files" section** (2h)
  - Show when query is empty
  - Load from LSSharedFileList + custom tracking
  - Limit to 10 items
  - Dependencies: 0.5.1

- [ ] **0.5.3 Add "Search Results" section** (1h)
  - Show when query has results
  - Dependencies: 0.5.1

- [ ] **0.5.4 Implement empty state** (2h)
  - Icon (📁)
  - Title: "No files found"
  - Description: "Try a different search term..."
  - Show when no results match query
  - Dependencies: 0.1.1

---

## Phase 1: File Actions & Enhancements (Week 3-4)

### 1.1 Missing File Actions (Priority: Critical)

- [ ] **1.1.1 Implement Copy File action** (2h)
  - Use NSFileManager to copy file to clipboard
  - Support multiple file selection
  - Add to actions panel with `Cmd+Shift+C`
  - Dependencies: None

- [ ] **1.1.2 Implement Move to Trash action** (2h)
  - Use NSFileManager's `trashItem(at:resultingItemURL:)`
  - Add confirmation for multiple files
  - Add undo support via notification
  - Shortcut: `Cmd+Backspace`
  - Dependencies: None

- [ ] **1.1.3 Implement Delete action** (2h)
  - Permanent delete with confirmation dialog
  - Use NSFileManager's `removeItem(at:)`
  - Show warning about irreversibility
  - Dependencies: None

- [ ] **1.1.4 Implement Open With action** (3h)
  - Show app picker for compatible apps
  - Use NSWorkspace's `urlsForApplications(toOpen:)`
  - Add "Other..." option to browse all apps
  - Shortcut: `Cmd+O`
  - Dependencies: None

- [ ] **1.1.5 Implement Rename action** (2h)
  - Inline rename in results list
  - Validate filename (no /, :, etc.)
  - Use NSFileManager's `moveItem(at:to:)`
  - Dependencies: None

- [ ] **1.1.6 Implement Move To action** (3h)
  - Show folder picker dialog
  - Support drag and drop
  - Recent destinations list
  - Dependencies: None

- [ ] **1.1.7 Implement Duplicate action** (1h)
  - Copy file with " copy" suffix
  - Handle name conflicts
  - Dependencies: 1.1.1

- [ ] **1.1.8 Implement Get Info action** (2h)
  - Show file metadata in detail view
  - Size, dates, permissions, type
  - Dependencies: None

- [ ] **1.1.9 Implement Compress action** (2h)
  - Create zip archive
  - Support folder compression
  - Show progress for large files
  - Dependencies: None

### 1.2 File Type Filtering Integration (Priority: High)

> Note: UI for filtering is in Phase 0. This phase integrates with search.

- [ ] **1.2.1 Define file type UTI mappings** (1h)
  - Map categories to UTType identifiers
  - Documents: public.document, public.pdf, etc.
  - Images: public.image
  - Videos: public.movie
  - Dependencies: 0.4.2

- [ ] **1.2.2 Integrate filter with Spotlight** (2h)
  - Build NSPredicate with UTI filter
  - Combine with name search predicate
  - Dependencies: 1.2.1

- [ ] **1.2.3 Integrate filter with custom index** (2h)
  - Filter by extension in SQLite query
  - Dependencies: 1.2.1

### 1.3 Quick Look Enhancement (Priority: Medium)

- [ ] **1.3.1 Improve Quick Look responsiveness** (2h)
  - Pre-load preview on selection
  - Cancel previous preview on navigation
  - Dependencies: None

- [ ] **1.3.2 Add Quick Look panel position** (1h)
  - Position relative to launcher window
  - Handle multi-monitor setup
  - Dependencies: None

- [ ] **1.3.3 Add Quick Look keyboard shortcuts** (1h)
  - Arrow keys to navigate while preview open
  - Cmd+Y toggle preview
  - Dependencies: None

### 1.4 External Drive Support (Priority: Medium)

- [ ] **1.4.1 Detect external drives** (2h)
  - Use DiskArbitration framework
  - List mounted volumes
  - Filter system volumes
  - Dependencies: None

- [ ] **1.4.2 Add Full Disk Access check** (2h)
  - Detect permission status
  - Show prompt to grant access
  - Link to System Preferences
  - Dependencies: None

- [ ] **1.4.3 Index external drives** (3h)
  - Add to search scopes when granted
  - Handle mount/unmount events
  - Dependencies: 1.4.1, 1.4.2

---

## Phase 2: Query Syntax (Week 5-6)

### 2.1 Natural Language Queries (Priority: High)

- [ ] **2.1.1 Implement query parser** (4h)
  - Parse "in folder" syntax
  - Extract file type from query
  - Handle quoted strings
  - Dependencies: None

- [ ] **2.1.2 Support location queries** (3h)
  - Parse `.txt in ~/Desktop`
  - Resolve `~`, `Documents`, `Downloads`, etc.
  - Dependencies: 2.1.1

- [ ] **2.1.3 Combine with Spotlight** (2h)
  - Build NSPredicate from parsed query
  - Set search scopes from location
  - Dependencies: 2.1.1, 2.1.2

### 2.2 Folder Prioritization (Priority: Medium)

- [ ] **2.2.1 Detect folder query syntax** (1h)
  - Check for trailing `/`
  - Dependencies: 2.1.1

- [ ] **2.2.2 Boost folder scores** (2h)
  - Multiply folder scores by 2x
  - Sort folders before files
  - Dependencies: 2.2.1

### 2.3 Parent Folder Search (Priority: Medium)

- [ ] **2.3.1 Parse parent folder syntax** (2h)
  - Extract folder from `docs/bar`
  - Support multiple levels `a/b/c`
  - Dependencies: 2.1.1

- [ ] **2.3.2 Filter results by parent** (2h)
  - Match any ancestor folder
  - Case-insensitive matching
  - Dependencies: 2.3.1

---

## Phase 3: Browsing Mode (Week 7-8)

### 3.1 Path Detection (Priority: Critical)

- [ ] **3.1.1 Detect browsing mode triggers** (2h)
  - Check for `/`, `~`, `~/` prefix
  - Check for absolute path pattern
  - Dependencies: None

- [ ] **3.1.2 Switch to browsing mode** (2h)
  - Update UI state
  - Change placeholder text
  - Show breadcrumb or current path
  - Dependencies: 3.1.1

- [ ] **3.1.3 Exit browsing mode** (1h)
  - Clear path prefix
  - Return to search mode
  - Dependencies: 3.1.1

### 3.2 Tab Navigation (Priority: Critical)

- [ ] **3.2.1 Implement Tab to enter folder** (3h)
  - Detect selected item is folder
  - Append to path, refresh listing
  - Handle symlinks (resolve)
  - Dependencies: 3.1.2

- [ ] **3.2.2 Implement Shift+Tab to go back** (2h)
  - Navigate to parent directory
  - Stop at root or scope boundary
  - Dependencies: 3.1.2

- [ ] **3.2.3 Tab on file expands full path** (1h)
  - Replace query with full path
  - Keep in browsing mode
  - Dependencies: 3.2.1

### 3.3 Directory Filtering (Priority: High)

- [ ] **3.3.1 Filter current directory** (2h)
  - Type to filter visible items
  - Fuzzy match file names
  - Dependencies: 3.1.2

- [ ] **3.3.2 Show filter indicator** (1h)
  - Display filter term
  - Clear button
  - Dependencies: 3.3.1

### 3.4 Environment Variables (Priority: Low)

- [ ] **3.4.1 Parse environment variable syntax** (2h)
  - Support `$HOME`, `${HOME}`, `$USER`
  - Dependencies: 3.1.1

- [ ] **3.4.2 Expand variables in path** (1h)
  - Use std::env::var
  - Handle missing variables gracefully
  - Dependencies: 3.4.1

---

## Phase 4: Custom Index (Week 9-10)

### 4.1 Tokenization Engine (Priority: Critical)

- [ ] **4.1.1 Implement whitespace tokenizer** (1h)
  - Split on spaces, tabs, newlines
  - Dependencies: None

- [ ] **4.1.2 Implement punctuation tokenizer** (1h)
  - Split on `-`, `_`, `.`
  - Keep extension separate
  - Dependencies: None

- [ ] **4.1.3 Implement camel case tokenizer** (2h)
  - Detect case transitions
  - Split `MyFile` → `my`, `file`
  - Dependencies: None

- [ ] **4.1.4 Implement ASCII folding** (1h)
  - Use unicode-normalization crate
  - é → e, ñ → n, etc.
  - Dependencies: None

- [ ] **4.1.5 Combine tokenizers** (1h)
  - Chain all tokenizers
  - Deduplicate tokens
  - Dependencies: 4.1.1-4.1.4

### 4.2 Background Indexing (Priority: Critical)

- [ ] **4.2.1 Create indexing service** (3h)
  - Spawn background thread
  - Progress reporting
  - Cancellation support
  - Dependencies: 4.1.5

- [ ] **4.2.2 Walk directory tree** (2h)
  - Use walkdir crate
  - Respect depth limits
  - Skip ignored directories
  - Dependencies: 4.2.1

- [ ] **4.2.3 Index files** (3h)
  - Extract metadata
  - Tokenize name and path
  - Store in inverted index
  - Dependencies: 4.2.1, 4.2.2

- [ ] **4.2.4 Show indexing progress** (2h)
  - Progress bar in UI
  - File count / total
  - Estimated time remaining
  - Dependencies: 4.2.1

### 4.3 File System Watcher (Priority: High)

- [ ] **4.3.1 Set up notify watcher** (2h)
  - Watch search scopes
  - Handle events (create, modify, delete, rename)
  - Dependencies: 4.2.1

- [ ] **4.3.2 Update index incrementally** (3h)
  - Add new files to index
  - Remove deleted files
  - Update modified files
  - Dependencies: 4.3.1, 4.2.3

- [ ] **4.3.3 Debounce rapid changes** (1h)
  - Batch updates within 500ms
  - Prevent index thrashing
  - Dependencies: 4.3.2

### 4.4 SQLite Storage (Priority: High)

- [ ] **4.4.1 Design index schema** (2h)
  - files table: id, path, name, ext, is_dir, size, mtime
  - tokens table: token, file_id
  - Create indexes
  - Dependencies: None

- [ ] **4.4.2 Implement index persistence** (3h)
  - Save index to SQLite
  - Load index on startup
  - Handle migrations
  - Dependencies: 4.4.1

- [ ] **4.4.3 Implement index queries** (3h)
  - Token-based search
  - Combine with scoring
  - Limit results
  - Dependencies: 4.4.1, 4.4.2

---

## Phase 5: Ignore Patterns (Week 11)

### 5.1 Pattern Parser (Priority: High)

- [ ] **5.1.1 Parse gitignore syntax** (3h)
  - Support wildcards (`*`, `**`, `?`)
  - Support negation (`!`)
  - Support directory patterns (`/`)
  - Dependencies: None

- [ ] **5.1.2 Load ignore files** (2h)
  - Find .gitignore, .ignore, .photonignore
  - Parse in order of priority
  - Dependencies: 5.1.1

### 5.2 Hierarchical Matching (Priority: High)

- [ ] **5.2.1 Apply patterns per directory** (3h)
  - Track active patterns per path
  - Inherit from parent directories
  - Dependencies: 5.1.2

- [ ] **5.2.2 Cache pattern results** (2h)
  - Memoize path matches
  - Invalidate on pattern change
  - Dependencies: 5.2.1

### 5.3 Exclude Action (Priority: Medium)

- [ ] **5.3.1 Add "Exclude from Index" action** (2h)
  - Find or create .photonignore
  - Append file pattern
  - Dependencies: 5.1.2

- [ ] **5.3.2 Update index after exclude** (1h)
  - Remove file from index
  - Show confirmation toast
  - Dependencies: 5.3.1, 4.3.2

---

## Phase 6: Preferences & Polish (Week 12)

### 6.1 Preferences UI (Priority: High)

- [ ] **6.1.1 Add File Search preferences section** (3h)
  - Search scopes list
  - Indexing options
  - Display options
  - Dependencies: None

- [ ] **6.1.2 Implement search scope editor** (3h)
  - Add/remove scopes
  - Folder picker
  - Show indexed file count per scope
  - Dependencies: 6.1.1

- [ ] **6.1.3 Add hotkey configuration** (2h)
  - Dedicated File Search hotkey
  - Default: `Cmd+Shift+F`
  - Dependencies: 6.1.1

### 6.2 Search Scope Management (Priority: Medium)

- [ ] **6.2.1 Persist scope changes** (1h)
  - Save to config file
  - Reload on preference change
  - Dependencies: 6.1.2

- [ ] **6.2.2 Trigger reindex on scope change** (1h)
  - Add new scope files
  - Remove out-of-scope files
  - Dependencies: 6.2.1, 4.2.1

### 6.3 Testing & Optimization (Priority: High)

- [ ] **6.3.1 Write unit tests** (4h)
  - Tokenizer tests
  - Query parser tests
  - Pattern matcher tests
  - Dependencies: All implementation tasks

- [ ] **6.3.2 Write integration tests** (3h)
  - End-to-end search tests
  - Browsing mode tests
  - File action tests
  - Dependencies: 6.3.1

- [ ] **6.3.3 Performance optimization** (3h)
  - Profile search queries
  - Optimize hot paths
  - Add result caching
  - Dependencies: 6.3.2

- [ ] **6.3.4 Documentation** (2h)
  - Update user manual
  - Add keyboard shortcut reference
  - Document query syntax
  - Dependencies: 6.3.3

---

## Summary

| Phase | Tasks | Estimated Hours |
|-------|-------|-----------------|
| Phase 0: UI Foundation | 19 | 42h |
| Phase 1: File Actions & Enhancements | 14 | 28h |
| Phase 2: Query Syntax | 8 | 16h |
| Phase 3: Browsing Mode | 11 | 19h |
| Phase 4: Custom Index | 14 | 27h |
| Phase 5: Ignore Patterns | 6 | 13h |
| Phase 6: Preferences & Polish | 9 | 22h |
| **Total** | **81** | **167h** |

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Split-view UI complexity | High | Incremental implementation, reuse existing patterns |
| Spotlight API limitations | High | Custom index fallback |
| Index size for large file systems | Medium | Limit scopes, lazy loading |
| File system watcher reliability | Medium | Periodic reindex fallback |
| Permission complexities | Medium | Clear user guidance |
| Detail panel preview performance | Medium | Lazy loading, caching |
| Performance on HDD | Low | Prioritize SSD, async I/O |

---

## Success Criteria

1. Split-view UI matches Raycast layout exactly (List + Detail panel)
2. All Raycast file search features implemented
3. Search latency <100ms
4. Browsing mode latency <50ms
5. Detail panel preview loads <200ms
6. Index builds in <60s for 100k files
7. All unit and integration tests passing
8. User testing feedback incorporated

---

*Tasks last updated: 2026-01-22*
