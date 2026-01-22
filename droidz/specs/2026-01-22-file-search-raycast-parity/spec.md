# File Search - Raycast Parity Specification

**Status:** Draft  
**Created:** 2026-01-22  
**Target Version:** v1.1.0  
**Priority:** High

---

## 1. Overview

This specification defines a comprehensive file search system for PhotonCast that achieves 100% feature parity with Raycast's file search capabilities. The implementation combines macOS Spotlight integration with an advanced custom indexing engine, browsing mode, and intelligent query parsing.

### 1.1 Goals

1. **Raycast Feature Parity** - Match all Raycast file search capabilities
2. **Performance** - Results in <100ms, browsing mode instant
3. **Discoverability** - Recent files, smart suggestions, natural language queries
4. **Power User Features** - Browsing mode, file type filtering, ignore patterns

### 1.2 Non-Goals

- Full-text content search (Phase 5 feature)
- Network/cloud file search
- Encrypted file search

---

## 2. Feature Comparison Matrix

| Feature | Raycast | PhotonCast (Current) | PhotonCast (Target) |
|---------|---------|---------------------|---------------------|
| Basic file search | ✅ | ✅ | ✅ |
| Recent files | ✅ | ✅ | ✅ |
| Quick Look preview | ✅ | ✅ | ✅ |
| Open file | ✅ | ✅ | ✅ |
| Reveal in Finder | ✅ | ✅ | ✅ |
| Copy path | ✅ | ✅ | ✅ |
| Copy file | ✅ | ❌ | ✅ |
| Delete/Move to Trash | ✅ | ❌ | ✅ |
| Open with specific app | ✅ | ❌ | ✅ |
| Natural language queries | ✅ | ❌ | ✅ |
| File type filtering | ✅ | ❌ | ✅ |
| Browsing mode | ✅ | ❌ | ✅ |
| Tab path completion | ✅ | ❌ | ✅ |
| Custom search scopes | ✅ | ❌ | ✅ |
| Ignore patterns | ✅ | ❌ | ✅ |
| External drive support | ✅ | ❌ | ✅ |
| Custom indexing engine | ✅ | ❌ | ✅ |
| Folder prioritization | ✅ | ❌ | ✅ |
| Parent folder search | ✅ | ❌ | ✅ |

---

## 3. Search Modes

### 3.1 Search Mode (Default)

Standard file search using Spotlight and custom index.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  📁  Search Files...                                          esc to exit  │
├─────────────────────────────────────────────────────────────────────────────┤
│    Recent Files                                                             │
│    ├── 📄  presentation.pdf          ~/Documents          2 hours ago      │
│    ├── 📄  budget.xlsx               ~/Documents/Work     Yesterday        │
│    ├── 📁  Projects                  ~/Developer          3 days ago       │
│    └── 📄  notes.md                  ~/Desktop            1 week ago       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Browsing Mode

Triggered when query starts with `/`, `~`, `~/`, or an absolute path.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  📁  ~/Documents/                                             esc to exit  │
├─────────────────────────────────────────────────────────────────────────────┤
│    📁  Work/                                                                │
│    📁  Personal/                                                            │
│    📄  resume.pdf                                                           │
│    📄  notes.txt                                                            │
│                                                                             │
│    Tab to enter folder • Shift+Tab to go back • Type to filter             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Query Syntax

### 4.1 Basic Queries

| Query | Description | Example Matches |
|-------|-------------|-----------------|
| `foo` | Entries starting with "foo" in name components | `foo.pdf`, `My Foo Bar`, `MyFooDocument` |
| `foo bar` | Entries containing both terms (AND) | `foobar.txt`, `bar_foo.md` |
| `"exact phrase"` | Exact phrase match | `exact phrase.doc` |

### 4.2 File Type Filtering

| Query | Description | Example Matches |
|-------|-------------|-----------------|
| `foo .pdf` | PDF files with "foo" in name | `foo.pdf`, `my_foo_report.pdf` |
| `.txt` | All text files | `*.txt` |
| `.pdf certificate` | PDFs with "certificate" in name | `certificate.pdf`, `my_certificate_2024.pdf` |

### 4.3 Folder Queries

| Query | Description | Example Matches |
|-------|-------------|-----------------|
| `foo/` | Prioritize folders matching "foo" | `foo/` folder ranked higher |
| `docs/bar` | "bar" entries inside "docs" folder | `~/Documents/bar.txt` |
| `projects/config.json` | Narrow search to parent folder | `~/projects/app/config.json` |

### 4.4 Location Queries (Natural Language)

| Query | Description | Example Matches |
|-------|-------------|-----------------|
| `.txt in ~/Desktop` | Text files in specific folder | `~/Desktop/*.txt` |
| `report in Documents` | Files named "report" in Documents | `~/Documents/*report*` |
| `pdf in Downloads` | PDFs in Downloads folder | `~/Downloads/*.pdf` |

### 4.5 Browsing Mode Triggers

| Query | Mode |
|-------|------|
| `/` | Browse from root |
| `~` or `~/` | Browse from home directory |
| `/Users/...` | Browse absolute path |
| `$HOME/...` | Environment variable expansion |
| `${HOME}/...` | Environment variable expansion (alt syntax) |

---

## 5. Keyboard Shortcuts

### 5.1 Search Mode Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Open file with default app |
| `Cmd+Enter` | Reveal in Finder |
| `Cmd+O` | Open with... (app picker) |
| `Cmd+Y` | Quick Look preview |
| `Cmd+Shift+C` | Copy file |
| `Cmd+Shift+,` | Copy file path |
| `Cmd+C` | Copy file name |
| `Cmd+Backspace` | Move to Trash |
| `Cmd+P` | Filter by file type |
| `Cmd+K` | Open actions panel |
| `Tab` | Enter browsing mode / expand path |
| `Escape` | Exit file search mode |
| `↑` / `↓` | Navigate results |

### 5.2 Browsing Mode Shortcuts

| Shortcut | Action |
|----------|--------|
| `Tab` | Enter selected folder / expand path |
| `Shift+Tab` | Go to parent folder |
| `Enter` | Open selected item |
| `Cmd+Enter` | Reveal in Finder |
| Type text | Filter current directory |
| `Escape` | Exit browsing mode |

---

## 6. File Actions

### 6.1 Primary Actions

```rust
pub enum FileAction {
    /// Open file with default application
    Open,
    /// Reveal file in Finder
    RevealInFinder,
    /// Open Quick Look preview
    QuickLook,
    /// Open with specific application
    OpenWith { app_bundle_id: String },
    /// Copy file to clipboard
    CopyFile,
    /// Copy file path to clipboard
    CopyPath,
    /// Copy file name to clipboard
    CopyName,
    /// Move file to Trash
    MoveToTrash,
    /// Delete file permanently (with confirmation)
    Delete,
    /// Rename file
    Rename { new_name: String },
    /// Move file to different location
    MoveTo { destination: PathBuf },
    /// Duplicate file
    Duplicate,
    /// Get file info
    GetInfo,
    /// Compress file/folder
    Compress,
    /// Exclude from search index
    ExcludeFromIndex,
}
```

### 6.2 Actions Panel (Cmd+K)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Actions for "presentation.pdf"                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│    ▶  Open                                                         ↵        │
│    📁  Reveal in Finder                                           ⌘↵       │
│    👁  Quick Look                                                  ⌘Y       │
│    📋  Copy File                                                  ⌘⇧C      │
│    📋  Copy Path                                                  ⌘⇧,      │
│    📋  Copy Name                                                   ⌘C       │
│    ─────────────────────────────────────────────────────────────────────    │
│    📂  Open With...                                                ⌘O       │
│    📂  Move To...                                                            │
│    ✏️  Rename                                                                │
│    📄  Duplicate                                                             │
│    🗜  Compress                                                              │
│    ─────────────────────────────────────────────────────────────────────    │
│    🗑  Move to Trash                                               ⌘⌫       │
│    🚫  Exclude from Index                                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Custom Indexing Engine

### 7.1 Index Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           File Search Index                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                     │
│  │  Spotlight  │    │   Custom    │    │   Recent    │                     │
│  │    Index    │    │    Index    │    │    Files    │                     │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘                     │
│         │                  │                   │                            │
│         └────────────┬─────┴───────────────────┘                            │
│                      ▼                                                      │
│              ┌─────────────────┐                                            │
│              │  Query Engine   │                                            │
│              │  - Tokenizer    │                                            │
│              │  - Scorer       │                                            │
│              │  - Ranker       │                                            │
│              └─────────────────┘                                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Tokenization Rules

The custom indexer tokenizes file names using these rules:

1. **Whitespace splitting**: `my file.txt` → `["my", "file", "txt"]`
2. **Punctuation splitting**: `my-file_v2.txt` → `["my", "file", "v2", "txt"]`
3. **Camel case splitting**: `MyFileSearch.rs` → `["my", "file", "search", "rs"]`
4. **Lowercase normalization**: `README` → `readme`
5. **ASCII folding**: `résumé.pdf` → `resume`, `pdf`
6. **Path segment splitting**: `/Users/john/Documents` → `["users", "john", "documents"]`

```rust
pub struct FileTokenizer {
    /// Split on whitespace
    split_whitespace: bool,
    /// Split on punctuation (-, _, .)
    split_punctuation: bool,
    /// Split camelCase and PascalCase
    split_camel_case: bool,
    /// Convert to lowercase
    lowercase: bool,
    /// Fold accented characters to ASCII
    ascii_fold: bool,
}

impl FileTokenizer {
    pub fn tokenize(&self, input: &str) -> Vec<String> {
        // Implementation
    }
}
```

### 7.3 Index Storage

```rust
pub struct FileIndex {
    /// Token to file IDs mapping (inverted index)
    token_index: HashMap<String, Vec<FileId>>,
    /// File ID to metadata mapping
    files: HashMap<FileId, FileMetadata>,
    /// Recent files (LRU cache)
    recent_files: LruCache<PathBuf, Instant>,
    /// Index version for migrations
    version: u32,
}

pub struct FileMetadata {
    pub id: FileId,
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub is_directory: bool,
    pub size: u64,
    pub modified: SystemTime,
    pub created: SystemTime,
    pub tokens: Vec<String>,
}
```

### 7.4 Background Indexing

```rust
pub struct IndexingService {
    /// Folders to index
    search_scopes: Vec<PathBuf>,
    /// File system watcher for real-time updates
    watcher: RecommendedWatcher,
    /// Indexing progress
    progress: Arc<AtomicUsize>,
    /// Total files to index
    total: Arc<AtomicUsize>,
}

impl IndexingService {
    /// Start background indexing
    pub async fn start(&mut self) -> Result<()>;
    
    /// Get indexing progress (0.0 - 1.0)
    pub fn progress(&self) -> f32;
    
    /// Check if indexing is complete
    pub fn is_ready(&self) -> bool;
    
    /// Manually trigger reindex
    pub async fn reindex(&mut self) -> Result<()>;
}
```

---

## 8. Ignore Patterns

### 8.1 Supported Ignore Files

| File | Scope | Priority |
|------|-------|----------|
| `.gitignore` | Directory and children | Low |
| `.ignore` | Directory and children | Medium |
| `.photonignore` | Directory and children | High |
| `~/.config/photoncast/ignore` | Global | Lowest |

### 8.2 Pattern Format

Uses Git's gitignore pattern format:

```gitignore
# Ignore node_modules everywhere
node_modules/

# Ignore build outputs
target/
dist/
build/

# Ignore specific files
*.log
*.tmp
.DS_Store

# Ignore hidden folders except .config
.*
!.config/

# Negate pattern (include despite previous rules)
!important.log
```

### 8.3 Exclude from Index Action

When user selects "Exclude from Index":

1. Find nearest `.photonignore` file (or create in same directory)
2. Append file/folder pattern
3. Update index immediately
4. Show confirmation toast

---

## 9. Search Scopes Configuration

### 9.1 Default Scopes

```rust
pub fn default_search_scopes() -> Vec<PathBuf> {
    vec![
        dirs::home_dir().unwrap(),           // ~
        PathBuf::from("/Applications"),       // System apps
        PathBuf::from("/System/Applications"),// System apps
    ]
}
```

### 9.2 Custom Scopes (Preferences)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  File Search Preferences                                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Search Scopes                                                              │
│  ─────────────────────────────────────────────────────────────────────────  │
│    ✓  ~/                              (Home folder)                         │
│    ✓  /Applications                   (Applications)                        │
│    ☐  /Volumes/External               (External Drive)         + Add       │
│                                                                             │
│  Note: External drives require Full Disk Access permission                  │
│                                                                             │
│  Indexing                                                                   │
│  ─────────────────────────────────────────────────────────────────────────  │
│    Index hidden files         [ ]                                           │
│    Respect .gitignore         [✓]                                           │
│    Index on startup           [✓]                                           │
│                                                                             │
│  Display                                                                    │
│  ─────────────────────────────────────────────────────────────────────────  │
│    Show file size             [✓]                                           │
│    Show modified date         [✓]                                           │
│    Show full path             [ ]                                           │
│    Result limit               [50]                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 10. File Type Filtering

### 10.1 Filter Categories

| Category | Extensions |
|----------|------------|
| Documents | pdf, doc, docx, xls, xlsx, ppt, pptx, txt, rtf, odt, pages, numbers, key |
| Images | jpg, jpeg, png, gif, bmp, svg, webp, ico, tiff, heic, raw |
| Videos | mp4, mov, avi, mkv, wmv, flv, webm, m4v |
| Audio | mp3, wav, flac, aac, ogg, m4a, wma, aiff |
| Archives | zip, rar, 7z, tar, gz, bz2, xz, dmg, iso |
| Code | rs, js, ts, py, rb, go, java, c, cpp, h, swift, kt, cs, php, html, css, json, yaml, toml, md |
| Folders | (directories only) |

### 10.2 Filter UI (Cmd+P)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Filter by Type                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│    ○  All Files                                                             │
│    ○  Documents                                                             │
│    ○  Images                                                                │
│    ○  Videos                                                                │
│    ○  Audio                                                                 │
│    ○  Archives                                                              │
│    ○  Code                                                                  │
│    ○  Folders Only                                                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 11. Recent Files

### 11.1 Data Source

Recent files are gathered from:

1. **LSSharedFileList** - macOS recent documents
2. **Custom tracking** - Files opened via PhotonCast
3. **Application recent files** - Per-app recent documents

### 11.2 Recent Files Display

```rust
pub struct RecentFile {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: DateTime<Utc>,
    pub open_count: u32,
    pub source: RecentFileSource,
}

pub enum RecentFileSource {
    /// Opened via PhotonCast
    PhotonCast,
    /// macOS LSSharedFileList
    MacOSRecent,
    /// Application-specific recent
    Application { bundle_id: String },
}
```

### 11.3 Sorting

Recent files sorted by:
1. Last opened time (most recent first)
2. Open frequency (for ties)
3. Alphabetically (for ties)

---

## 12. External Drive Support

### 12.1 Permission Requirements

| Feature | Permission Required |
|---------|---------------------|
| Search home folder | None (default) |
| Search /Applications | None (default) |
| Search external drives | Full Disk Access |
| Search system folders | Full Disk Access |

### 12.2 External Drive Detection

```rust
pub fn detect_external_drives() -> Vec<ExternalDrive> {
    // Use diskutil or DiskArbitration framework
    // Filter to mounted, readable volumes
    // Exclude system volumes (Preboot, Recovery, VM)
}

pub struct ExternalDrive {
    pub name: String,
    pub mount_point: PathBuf,
    pub capacity: u64,
    pub available: u64,
    pub is_removable: bool,
    pub is_ejectable: bool,
}
```

---

## 13. Configuration

### 13.1 Config Schema

```toml
[file_search]
# Search scopes
search_scopes = ["~", "/Applications"]

# Custom scopes (additional)
custom_scopes = ["/Volumes/External"]

# Indexing options
index_hidden_files = false
respect_gitignore = true
index_on_startup = true

# Display options
show_file_size = true
show_modified_date = true
show_full_path = false
result_limit = 50

# Performance
max_index_size_mb = 500
indexing_threads = 2
query_timeout_ms = 100
```

### 13.2 Default Values

```rust
impl Default for FileSearchConfig {
    fn default() -> Self {
        Self {
            search_scopes: default_search_scopes(),
            custom_scopes: vec![],
            index_hidden_files: false,
            respect_gitignore: true,
            index_on_startup: true,
            show_file_size: true,
            show_modified_date: true,
            show_full_path: false,
            result_limit: 50,
            max_index_size_mb: 500,
            indexing_threads: 2,
            query_timeout_ms: 100,
        }
    }
}
```

---

## 14. Performance Requirements

| Metric | Target | Notes |
|--------|--------|-------|
| Search latency | <100ms | From keystroke to results |
| Browsing mode | <50ms | Directory listing |
| Quick Look | <200ms | Preview display |
| Initial indexing | <60s | For 100k files |
| Index update | <5s | Incremental updates |
| Memory (index) | <200MB | For 100k files |
| Disk (index) | <100MB | SQLite database |

---

## 15. Implementation Phases

### Phase 1: Core Enhancements (Week 1-2)
- [ ] Implement missing file actions (Copy, Delete, Open With)
- [ ] Add file type filtering (Cmd+P)
- [ ] Improve Quick Look integration
- [ ] Add external drive support

### Phase 2: Query Syntax (Week 3-4)
- [ ] Implement natural language queries
- [ ] Add folder prioritization (`foo/`)
- [ ] Add parent folder search (`docs/bar`)
- [ ] Support file type syntax (`.pdf certificate`)

### Phase 3: Browsing Mode (Week 5-6)
- [ ] Implement path detection triggers
- [ ] Add Tab/Shift+Tab navigation
- [ ] Add directory filtering
- [ ] Support environment variables

### Phase 4: Custom Index (Week 7-8)
- [ ] Build tokenization engine
- [ ] Implement background indexing
- [ ] Add file system watcher
- [ ] Create SQLite index storage

### Phase 5: Ignore Patterns (Week 9)
- [ ] Parse .gitignore, .ignore, .photonignore
- [ ] Implement hierarchical pattern matching
- [ ] Add "Exclude from Index" action

### Phase 6: Preferences & Polish (Week 10)
- [ ] Build preferences UI
- [ ] Add search scope management
- [ ] Performance optimization
- [ ] Testing and bug fixes

---

## 16. Testing Strategy

### 16.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_tokenizer_whitespace() {
        let tokenizer = FileTokenizer::default();
        assert_eq!(tokenizer.tokenize("my file"), vec!["my", "file"]);
    }
    
    #[test]
    fn test_tokenizer_camel_case() {
        let tokenizer = FileTokenizer::default();
        assert_eq!(tokenizer.tokenize("MyFileSearch"), vec!["my", "file", "search"]);
    }
    
    #[test]
    fn test_query_parser_file_type() {
        let query = FileQuery::parse("report .pdf");
        assert_eq!(query.terms, vec!["report"]);
        assert_eq!(query.file_type, Some("pdf"));
    }
    
    #[test]
    fn test_query_parser_folder_priority() {
        let query = FileQuery::parse("downloads/");
        assert!(query.prioritize_folders);
    }
}
```

### 16.2 Integration Tests

- Search returns correct results for various query types
- Browsing mode navigates correctly
- File actions execute properly
- Indexing completes without errors
- Ignore patterns work correctly

---

## 17. Security Considerations

1. **Sandboxing** - Respect macOS sandbox restrictions
2. **Permissions** - Request only necessary permissions
3. **No sensitive data** - Don't index password files, keychains
4. **Secure delete** - Use NSFileManager's secure delete
5. **No network** - Index is local only, never transmitted

---

## 18. Accessibility

1. **VoiceOver support** - All UI elements labeled
2. **Keyboard navigation** - Full keyboard accessibility
3. **High contrast** - Respect system preferences
4. **Reduce motion** - Respect system preferences
5. **Screen reader** - Announce results and actions

---

## 19. References

- [Raycast File Search (Mac)](https://www.raycast.com/core-features/file-search)
- [Raycast File Search Manual](https://manual.raycast.com/core)
- [Raycast Windows File Search](https://manual.raycast.com/windows/file-search)
- [Apple Spotlight Documentation](https://developer.apple.com/documentation/coreservices/file_metadata/mdquery)
- [NSMetadataQuery](https://developer.apple.com/documentation/foundation/nsmetadataquery)

---

*Specification last updated: 2026-01-22*
