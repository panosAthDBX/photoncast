# PhotonCast Code Review Fixes — Specification

## 1. Overview

This specification covers all findings from the PhotonCast code review, organized into four work streams:

1. **Security Hardening** — 9 findings addressing extension loading safety, path traversal, action validation, and unsafe code
2. **Performance Optimization** — 15 findings addressing blocking I/O, redundant allocations, and algorithmic inefficiencies
3. **Structural Refactoring** — 9 findings addressing god functions/structs, code duplication, and SRP violations
4. **Code Cleanup** — 6 findings addressing dead code, magic numbers, and lint suppressions

**Tech Stack:** Rust desktop app using GPUI framework with a native extension system (`abi_stable` dylibs).

**Scope:** All changes target the `feat/native-extension-system` branch. No public API changes to the extension API crate unless noted.

---

## 2. Work Stream 1: Security Hardening

### 2.1 Overview & Motivation

The native extension system loads arbitrary dylibs into the host process. Without verification, a malicious extension can execute arbitrary code, traverse paths, and access system resources. These findings harden the extension loading pipeline and action execution.

### 2.2 Finding #1 — Dylib Loading Without Code Signing Verification

**Severity:** HIGH

**File:** `crates/photoncast-core/src/extensions/manager.rs`

**Current behavior:**

Extensions are loaded via `ExtensionLoader::load(&entry_path)` (which calls `abi_stable::lib_header_from_path`) without any authenticity verification. The `load_and_activate` method (line ~213) and `reload_extension` method (line ~333) both load dylibs from disk with no signature or hash check:

```rust
// load_and_activate, ~line 213
let entry_path = resolve_entry_path(&record.manifest, None);
let library = ExtensionLoader::load(&entry_path)?;
```

A malicious `.dylib` placed in the extensions directory executes with full app privileges.

**Desired behavior:**

Before loading any dylib, verify its authenticity using one or both of:
- **Code signature verification** via `codesign --verify` (macOS)
- **Allowlist of known SHA-256 hashes** stored in a verified manifest

**Implementation approach:**
1. Add a `verify_extension_integrity(path: &Path) -> Result<(), SecurityError>` function
2. On macOS, shell out to `codesign --verify --deep --strict <path>` and check exit code
3. Maintain an optional `trusted_hashes.json` allowlist in the app data directory
4. In dev mode, skip verification but log a warning
5. Call verification before `ExtensionLoader::load()` in both `load_and_activate` and `reload_extension`

**Acceptance criteria:**
- [ ] Unsigned dylibs are rejected with a clear error message (outside dev mode)
- [ ] Dev mode logs a warning but allows loading
- [ ] Hash allowlist is checked when present
- [ ] Unit test verifies rejection of tampered dylib path
- [ ] No regression in extension load time (verification < 100ms)

**Risk:** Medium — could break existing development workflows if not gated behind dev mode properly.

---

### 2.3 Finding #2 — Path Traversal in Extension Manifests

**Severity:** MEDIUM

**File:** `crates/photoncast-core/src/extensions/manager.rs` — `resolve_entry_path()` (line ~533)

**Current behavior:**

The `resolve_entry_path` function joins the manifest's `entry.path` directly without sanitization:

```rust
fn resolve_entry_path(manifest: &ExtensionManifest, override_path: Option<&Path>) -> PathBuf {
    if let Some(path) = override_path {
        return path.to_path_buf();
    }
    let base_dir = manifest
        .directory
        .as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("."));
    base_dir.join(&manifest.entry.path)
}
```

An `extension.toml` with `entry.path = "../../malicious.dylib"` would resolve outside the extension directory.

**Desired behavior:**

Canonicalize the resolved path and verify it starts with the extension directory:

```rust
fn resolve_entry_path(manifest: &ExtensionManifest, override_path: Option<&Path>) -> Result<PathBuf, ExtensionManagerError> {
    let base_dir = manifest.directory.as_ref()
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("."));
    
    let resolved = base_dir.join(&manifest.entry.path);
    let canonical = resolved.canonicalize()
        .map_err(|e| ExtensionManagerError::PathTraversal { path: resolved.clone() })?;
    let canonical_base = base_dir.canonicalize()
        .map_err(|e| ExtensionManagerError::PathTraversal { path: base_dir.to_path_buf() })?;
    
    if !canonical.starts_with(&canonical_base) {
        return Err(ExtensionManagerError::PathTraversal { path: resolved });
    }
    Ok(canonical)
}
```

**Acceptance criteria:**
- [ ] `entry.path = "../../malicious.dylib"` is rejected with `PathTraversal` error
- [ ] Symlinks pointing outside the extension dir are rejected
- [ ] Normal relative paths (e.g., `target/release/libext.dylib`) still work
- [ ] Unit tests cover traversal attempts with `..`, absolute paths, and symlinks

**Risk:** Low — straightforward path validation.

---

### 2.4 Finding #3 — Action Execution Bypasses Host Permission Checks

**Severity:** MEDIUM

**File:** `crates/photoncast/src/extension_views/actions.rs` — `execute_action()` (lines 28–75)

**Current behavior:**

`OpenUrl`, `OpenFile`, `RevealInFinder` pass extension-supplied strings directly to `open::that()` and `std::process::Command` without validation:

```rust
ActionHandler::OpenUrl(url) => {
    let url = url.to_string();
    let _ = open::that(&url);
    should_close = true;
},
ActionHandler::OpenFile(path) => {
    let path = path.to_string();
    let _ = open::that(&path);
    should_close = true;
},
```

A malicious extension could open `file:///etc/passwd`, arbitrary URLs, or trigger protocol handlers.

**Desired behavior:**

1. **URL validation**: For `OpenUrl`, parse with `url::Url` and restrict to `http`/`https` schemes. Reject `file://`, `javascript:`, custom schemes unless explicitly allowed.
2. **Path validation**: For `OpenFile` and `RevealInFinder`, verify the path exists as a regular file/directory and doesn't point to sensitive system locations.
3. **Logging**: Log all action executions with extension ID for audit trail.

**Acceptance criteria:**
- [ ] `OpenUrl` rejects non-HTTP(S) URLs
- [ ] `OpenFile` validates the path exists and is a regular file
- [ ] `RevealInFinder` validates the path exists
- [ ] All action executions are logged with extension ID
- [ ] Unit tests cover malicious URL schemes and non-existent paths

**Risk:** Low — may need `url` crate dependency (check if already present).

---

### 2.5 Finding #4 — Unsanitized Process Arguments in RevealInFinder

**Severity:** MEDIUM

**File:** `crates/photoncast/src/extension_views/actions.rs` (lines 40–44)

**Current behavior:**

```rust
ActionHandler::RevealInFinder(path) => {
    let path = path.to_string();
    let _ = std::process::Command::new("open")
        .args(["-R", &path])
        .spawn();
    should_close = true;
},
```

The path is passed directly as an argument to the `open` command. While `args()` is safer than shell execution, a path like `/dev/null` or a symlink to a sensitive location could be problematic.

**Desired behavior:**

Validate the path is an existing filesystem path before passing to `open -R`. This is subsumed by Finding #3's path validation but specifically:

```rust
ActionHandler::RevealInFinder(path) => {
    let path_str = path.to_string();
    let path = std::path::Path::new(&path_str);
    if path.exists() && !is_sensitive_path(path) {
        let _ = std::process::Command::new("open")
            .args(["-R", &path_str])
            .spawn();
    } else {
        tracing::warn!(path = %path_str, "RevealInFinder: invalid or sensitive path");
    }
    should_close = true;
},
```

**Acceptance criteria:**
- [ ] Non-existent paths are rejected with a warning log
- [ ] Sensitive system paths (`/etc`, `/System`, etc.) are blocked
- [ ] Valid user-accessible paths work normally

**Risk:** Low.

---

### 2.6 Finding #5 — Clipboard Content Exposure

**Severity:** MEDIUM

**File:** `crates/photoncast/src/extension_views/actions.rs` (line 51)

**Current behavior:**

```rust
ActionHandler::CopyToClipboard(text) => {
    let text = text.to_string();
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
    should_close = true;
},
```

Extension-supplied content is written to the system clipboard without any logging, size limit, or user confirmation.

**Desired behavior:**

1. Log clipboard writes at `debug` level (content truncated to prevent secret leakage in logs)
2. Enforce a reasonable size limit (e.g., 1MB)
3. Optionally show a brief toast notification ("Copied to clipboard")

**Acceptance criteria:**
- [ ] Clipboard writes are logged (truncated content, extension ID)
- [ ] Content exceeding 1MB is rejected
- [ ] Toast notification shown on successful copy (using existing toast system)

**Risk:** Low.

---

### 2.7 Finding #6 — Unsafe Send+Sync Implementations

**Severity:** LOW

**File:** `crates/photoncast-core/src/extensions/storage.rs` (lines 34–35), `crates/photoncast-core/src/extensions/host.rs` (lines 41–42)

**Current behavior:**

```rust
// storage.rs:34-35
unsafe impl Send for ExtensionStorageImpl {}
unsafe impl Sync for ExtensionStorageImpl {}

// host.rs:41-42
unsafe impl Send for ExtensionHostServices {}
unsafe impl Sync for ExtensionHostServices {}
```

Both have justification comments above them, but the safety invariants should be formally documented and audited.

**Desired behavior:**

1. Audit all fields of `ExtensionStorageImpl` and `ExtensionHostServices` for thread safety
2. Replace `unsafe impl` with derived `Send`/`Sync` where possible (e.g., if all fields are already `Send`+`Sync`)
3. If `unsafe impl` must remain, add detailed `// SAFETY:` comments documenting which fields require it and why
4. Consider wrapping non-Send fields in `Arc<Mutex<>>` to derive Send/Sync safely

**Acceptance criteria:**
- [ ] Each `unsafe impl` has a `// SAFETY:` comment listing all fields and their thread-safety
- [ ] Any fields that can be made `Send`/`Sync` through wrapping are wrapped
- [ ] Miri or thread sanitizer test confirming no data races (if feasible)

**Risk:** Medium — changing synchronization wrappers could affect performance.

---

### 2.8 Finding #7 — Y2038 Timestamp Truncation

**Severity:** LOW

**File:** `crates/photoncast-core/src/storage/database.rs` (line ~124)

**Current behavior:**

```rust
fn record_version(&self, version: i32) -> Result<()> {
    let conn = self.conn.lock();
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
        [&version, &(now as i32)],  // ← truncates i64 to i32
    )
    .context("failed to record schema version")?;
    Ok(())
}
```

`Utc::now().timestamp()` returns `i64`. Casting to `i32` will overflow on **January 19, 2038**. While this is in schema migration code (low frequency), it sets a bad precedent.

**Desired behavior:**

Change `now as i32` to store the full `i64` timestamp. Update the `schema_version` table DDL if it uses `INTEGER` (SQLite `INTEGER` can store 64-bit values, so the DDL may already be fine).

**Acceptance criteria:**
- [ ] `applied_at` stored as `i64` (no truncation)
- [ ] Database schema is compatible (SQLite INTEGER handles this natively)
- [ ] Grep the codebase for other `as i32` timestamp casts and fix any found

**Risk:** Low — SQLite handles i64 transparently.

---

### 2.9 Finding #8 — `expand_tilde` Incomplete

**Severity:** LOW

**File:** `crates/photoncast-core/src/utils/paths.rs` (lines 56–62)

**Current behavior:**

```rust
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with('~') {
        dirs::home_dir()
            .map(|h: PathBuf| h.join(&path[2..]))  // ← skips 2 chars, assumes ~/
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}
```

A bare `~` (without trailing `/`) would panic or produce incorrect results because `&path[2..]` on a 1-character string is an empty slice at best. Also, `~username` syntax is not handled.

**Desired behavior:**

```rust
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(path))
    } else if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}
```

**Acceptance criteria:**
- [ ] `expand_tilde("~")` returns the home directory
- [ ] `expand_tilde("~/Documents")` returns `$HOME/Documents`
- [ ] `expand_tilde("/absolute/path")` returns unchanged
- [ ] `expand_tilde("")` returns empty path
- [ ] Unit tests cover all cases

**Risk:** Low.

---

### 2.10 Finding #9 — No Extension Sandboxing

**Severity:** LOW (architectural, long-term)

**Current behavior:** Extensions run in-process with full host privileges. The permissions system (`permissions.rs`) provides consent UI but no runtime enforcement boundary.

**Desired behavior (long-term):** This is an architectural concern. For now, document the risk and plan for:
- Process-based isolation (separate process per extension)
- Or WASM-based sandboxing
- Or capability-based restriction at the host API level

**Acceptance criteria:**
- [ ] Add `// TODO(security): Extension sandboxing` comment in `manager.rs`
- [ ] Create a tracking issue for extension sandboxing design
- [ ] Document the current risk in extension developer documentation

**Risk:** N/A — documentation only for now.

---

## 3. Work Stream 2: Performance Optimization

### 3.1 Overview & Motivation

Several hot paths contain blocking I/O, redundant allocations, and suboptimal algorithms that cause UI freezes and wasted CPU cycles. The most critical findings affect the screenshots extension (blocking image I/O) and search ranking (per-result allocations).

### 3.2 Finding #10 — Synchronous Image I/O Blocking UI

**Severity:** CRITICAL

**File:** `crates/photoncast-ext-screenshots/src/lib.rs` (lines 67–87)

**Current behavior:**

The `get_or_create_thumbnail` function calls `image::open(path)` and `thumbnail.save()` synchronously on the command handler thread:

```rust
let img = match image::open(path) {
    Ok(img) => img,
    Err(_) => return None,
};

let (width, height) = img.dimensions();
if width <= THUMBNAIL_SIZE && height <= THUMBNAIL_SIZE {
    return Some(path.clone());
}

let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);

if thumbnail.save(&thumbnail_path).is_err() {
    return None;
}
```

For a folder with 50+ large screenshots, this blocks the UI for several seconds.

**Desired behavior:**

1. Return a placeholder/loading state immediately
2. Spawn thumbnail generation on a background thread pool
3. Cache generated thumbnails (already partially done with `thumbnail_path` check)
4. Notify the UI to refresh when thumbnails are ready

**Implementation approach:**
- Use `std::thread::spawn` or `rayon::spawn` for thumbnail generation
- Return items immediately with a `thumbnail: None` state
- Use a callback or channel to notify when thumbnails are ready
- Consider generating thumbnails lazily (only for visible items)

**Acceptance criteria:**
- [ ] Screenshots list appears instantly (< 100ms) even with 100+ images
- [ ] Thumbnails load asynchronously and appear as they're generated
- [ ] Previously cached thumbnails display immediately
- [ ] No UI freezes during thumbnail generation

**Risk:** Medium — requires changes to the extension's command handler lifecycle.

---

### 3.3 Finding #11 — Full Directory Scan on Every Command Invocation

**Severity:** CRITICAL

**File:** `crates/photoncast-ext-screenshots/src/lib.rs` (lines 287–312)

**Current behavior:**

`scan_screenshots` performs a full `read_dir` + metadata fetch + sort on every invocation:

```rust
fn scan_screenshots(folder: &str) -> Vec<Screenshot> {
    let path = if folder.starts_with('~') {
        dirs::home_dir()
            .map(|home| home.join(&folder[2..]))
            .unwrap_or_else(|| PathBuf::from(folder))
    } else {
        PathBuf::from(folder)
    };

    let mut screenshots = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    if let Some(screenshot) = Screenshot::new(file_path, metadata.len(), modified) {
                        screenshots.push(screenshot);
                    }
                }
            }
        }
    }
    screenshots.sort_by(|a, b| b.modified.cmp(&a.modified));
    screenshots
}
```

**Desired behavior:**

1. Cache the directory listing with a TTL (e.g., 5 seconds)
2. Use filesystem events (`kqueue`/`FSEvents`) for invalidation when possible
3. Only re-sort when the cache is refreshed

**Acceptance criteria:**
- [ ] Repeated command invocations within 5s return cached results
- [ ] Cache is invalidated when files change (add/delete)
- [ ] Initial scan still works correctly

**Risk:** Low.

---

### 3.4 Finding #12 — Blocking `sips` Process Spawn for Icon Extraction

**Severity:** CRITICAL

**File:** `crates/photoncast/src/launcher.rs` (lines ~1000, ~1038)

**Current behavior:**

`extract_icon_to_cache` spawns `sips` synchronously via `.output()` (which blocks until completion):

```rust
let output = std::process::Command::new("sips")
    .args([
        "-s", "format", "png",
        "-z", "64", "64",
        &icns_path.to_string_lossy(),
        "--out",
        &cache_path.to_string_lossy(),
    ])
    .output();  // ← blocking
```

During startup indexing (~200 apps), this spawns ~200 blocking `sips` processes.

**Desired behavior:**

1. Use async process spawning (`tokio::process::Command` or `smol::process::Command`)
2. Batch icon extractions with limited concurrency (e.g., `futures::stream::buffer_unordered(8)`)
3. Alternatively, use `NSWorkspace` API directly (already available via `objc2`) to get icons without spawning processes

**Acceptance criteria:**
- [ ] Icon extraction does not block the UI thread
- [ ] Startup indexing completes 2-3x faster
- [ ] Icons appear progressively as they're extracted
- [ ] Fallback to `sips` if native API unavailable

**Risk:** Medium — platform-specific changes.

---

### 3.5 Finding #13 — `to_lowercase()` Called Per-Result in Ranking Hot Path

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/search/ranking.rs` (around the ranking methods)

**Current behavior:**

The ranker calls `to_lowercase()` on result titles during comparison, allocating 2 new strings per search result in the ranking loop.

**Desired behavior:**

Pre-compute lowercased titles before the ranking loop:

```rust
// Before ranking
let lowered: Vec<String> = results.iter().map(|r| r.title.to_lowercase()).collect();
// Use lowered[i] in comparisons instead of results[i].title.to_lowercase()
```

**Acceptance criteria:**
- [ ] Zero string allocations inside the ranking comparison loop
- [ ] Search result ranking produces identical results
- [ ] Benchmark shows measurable improvement for 100+ results

**Risk:** Low.

---

### 3.6 Finding #14 — `to_lowercase()` Inside Sort Comparator

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/search/ranking.rs` (tiebreaker sort)

**Current behavior:**

The tiebreaker sort comparator calls `to_lowercase()` inside the closure, causing O(n log n) string allocations.

**Desired behavior:**

Pre-compute lowercased titles and use Schwartzian transform (decorate-sort-undecorate):

```rust
let mut indexed: Vec<(usize, String)> = results.iter()
    .enumerate()
    .map(|(i, r)| (i, r.title.to_lowercase()))
    .collect();
indexed.sort_by(|(_, a), (_, b)| a.cmp(b));
// Reorder results based on sorted indices
```

**Acceptance criteria:**
- [ ] String allocations reduced from O(n log n) to O(n)
- [ ] Tiebreaker sort produces identical results
- [ ] Unit tests verify ordering

**Risk:** Low.

---

### 3.7 Finding #15 — New FuzzyMatcher Created Per Search Call

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/extensions/manager.rs` — `search()` method (line ~385)

**Current behavior:**

```rust
pub fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
    // ...
    let mut matcher = crate::search::fuzzy::FuzzyMatcher::default();
    let mut command_matcher = crate::search::fuzzy::FuzzyMatcher::default();
    // ...
}
```

Two new matchers are allocated on every keystroke.

**Desired behavior:**

Cache matchers in the `ExtensionManager` struct or use `thread_local!` storage:

```rust
pub struct ExtensionManager {
    // ... existing fields
    search_matcher: crate::search::fuzzy::FuzzyMatcher,
    command_matcher: crate::search::fuzzy::FuzzyMatcher,
}
```

**Acceptance criteria:**
- [ ] FuzzyMatcher is reused across search calls
- [ ] Search results are identical
- [ ] Reduced allocation pressure measurable via benchmarks

**Risk:** Low — need to verify FuzzyMatcher is reusable (no stale state between calls).

---

### 3.8 Finding #16 — LauncherWindow Struct Cache Pressure

**Severity:** MAJOR

**File:** `crates/photoncast/src/launcher.rs` (lines 120–260)

**Current behavior:**

`LauncherWindow` has 50+ fields spanning search state, animation, calculator, calendar, app management, toast notifications, uninstall preview, auto-quit settings, and window management. This is a performance concern because the entire struct must fit in cache lines during hot-path operations.

**Desired behavior:**

Split into sub-structs grouping related fields:

```rust
struct LauncherWindow {
    search: SearchState,           // query, cursor, results
    animation: AnimationState,     // animation_state, animation_start, etc.
    calculator: CalculatorState,   // calculator_command, runtime, result
    app_management: AppManagementState,  // app_manager, uninstall_preview, auto_quit
    ui: UIState,                   // focus_handle, scroll_handle, toast
    // ... shared services
}
```

This also addresses Finding #26 (SRP violation) — see Work Stream 3.

**Acceptance criteria:**
- [ ] Hot-path fields (search query, selected index, results) are in a compact sub-struct
- [ ] Cold-path fields (uninstall preview, auto-quit) are in separate sub-structs
- [ ] No behavioral changes
- [ ] All existing tests pass

**Risk:** Medium — large refactor touching many methods.

---

### 3.9 Finding #17 — Cloning Entire filtered_items on Every Search Filter

**Severity:** MAJOR

**File:** `crates/photoncast/src/extension_views/list_view.rs` (lines 207–222)

**Current behavior:**

```rust
fn apply_search_filter(&mut self) {
    if self.search_query.is_empty() {
        self.filtered_items = self.flat_items.clone();  // ← full clone
    } else {
        let query_lower = self.search_query.to_lowercase();
        self.filtered_items = self
            .flat_items
            .iter()
            .filter(|item| {
                item.item.title.to_lowercase().contains(&query_lower)
                    || item.item.subtitle.as_ref()
                        .map_or(false, |s| s.to_lowercase().contains(&query_lower))
            })
            .cloned()  // ← clones each matching item
            .collect();
    }
}
```

Every keystroke clones all matching `FlatListItem`s (which contain full `ListItem` with strings, icons, accessories).

**Desired behavior:**

Store indices into `flat_items` instead of cloning:

```rust
fn apply_search_filter(&mut self) {
    if self.search_query.is_empty() {
        self.filtered_indices = (0..self.flat_items.len()).collect();
    } else {
        let query_lower = self.search_query.to_lowercase();
        self.filtered_indices = self.flat_items.iter()
            .enumerate()
            .filter(|(_, item)| /* ... */)
            .map(|(i, _)| i)
            .collect();
    }
}
```

**Acceptance criteria:**
- [ ] `filtered_items: Vec<FlatListItem>` replaced with `filtered_indices: Vec<usize>`
- [ ] All rendering code updated to index into `flat_items`
- [ ] No behavioral changes
- [ ] Reduced memory allocation per keystroke

**Risk:** Medium — requires updating all call sites that read `filtered_items`.

---

### 3.10 Finding #18 — `flat_results` Fully Cloned from SearchResults

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/ui/results_list.rs` (line 73)

**Current behavior:**

```rust
pub fn set_results(&mut self, results: SearchResults) {
    self.grouped_results = results.grouped();
    self.flat_results = results.iter().cloned().collect();  // ← clones every SearchResult
    self.search_results = results;
    self.selected_index = 0;
    self.scroll_offset = 0.0;
}
```

Every `SearchResult` is cloned on every search update.

**Desired behavior:**

Use `Arc<SearchResult>` or indices into the `SearchResults` collection:

```rust
self.flat_results = results.iter().map(Arc::clone).collect();
// Or: store indices and reference search_results directly
```

**Acceptance criteria:**
- [ ] `SearchResult` wrapped in `Arc` or referenced by index
- [ ] No deep cloning on search update
- [ ] No behavioral changes

**Risk:** Medium — `Arc` wrapping requires changes to `SearchResult` usage patterns.

---

### 3.11 Finding #19 — Quadratic Deduplication in Recent Files Merge

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/search/spotlight/prefetch.rs` (lines ~290)

**Current behavior:**

```rust
for result in results {
    if !files.iter().any(|f| f.path == result.path) {
        files.push(result);
    }
}
```

This is O(n*m) where n = new results and m = existing files. For large result sets (1000+), this becomes slow.

**Desired behavior:**

Use a `HashSet<PathBuf>` for O(1) deduplication:

```rust
let mut seen: HashSet<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
for result in results {
    if seen.insert(result.path.clone()) {
        files.push(result);
    }
}
```

**Acceptance criteria:**
- [ ] Deduplication uses `HashSet` for O(n+m) complexity
- [ ] Results are identical to current behavior
- [ ] Performance improvement measurable for 500+ results

**Risk:** Low.

---

### 3.12 Finding #20 — Dead Code: `_match_ranges` Computation

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/ui/result_item.rs` (lines 291–301)

**Current behavior:**

```rust
impl From<&crate::search::SearchResult> for ResultItem {
    fn from(result: &crate::search::SearchResult) -> Self {
        let _match_ranges: Vec<Range<usize>> = result  // ← computed but unused
            .match_indices
            .windows(2)
            .filter_map(|w| {
                if w[1] == w[0] + 1 { None }
                else { Some(w[0]..w[1]) }
            })
            .collect();

        // Convert match indices to ranges (consecutive indices form ranges)
        let ranges = indices_to_ranges(&result.match_indices);  // ← this is the one actually used
        // ...
    }
}
```

`_match_ranges` is computed but never used — `indices_to_ranges()` is called separately and its result is used.

**Desired behavior:**

Remove the dead `_match_ranges` computation entirely.

**Acceptance criteria:**
- [ ] `_match_ranges` computation removed
- [ ] No behavioral change
- [ ] No compiler warnings

**Risk:** None.

---

### 3.13 Finding #21 — Icon Cache Uses Manual LRU Scan

**Severity:** MINOR

**File:** `crates/photoncast-core/src/indexer/icons.rs` (lines 176–186)

**Current behavior:**

```rust
fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
    if let Some((key, entry)) = cache
        .iter()
        .min_by_key(|(_, entry)| entry.access_order)
        .map(|(k, v)| (k.clone(), v.icon.cached_path.clone()))
    {
        debug!("Evicting LRU icon: {}", key);
        cache.remove(&key);
        if let Err(e) = std::fs::remove_file(&entry) {
            // ...
```

O(n) scan for every eviction. The `lru` crate is already in the project's dependency tree.

**Desired behavior:**

Replace `HashMap<String, CacheEntry>` with `lru::LruCache<String, Arc<LazyIcon>>`.

**Acceptance criteria:**
- [ ] Cache uses `lru::LruCache`
- [ ] O(1) eviction
- [ ] Same behavior (LRU eviction, capacity limit)

**Risk:** Low.

---

### 3.14 Finding #22 — Clipboard `has_type` Iterates Full Type Array Per Check

**Severity:** MINOR

**File:** `crates/photoncast-clipboard/src/monitor.rs` (lines 475–481, called at lines 244–268)

**Current behavior:**

```rust
fn has_type(
    types: &objc2_foundation::NSArray<objc2_foundation::NSString>,
    type_name: &str,
) -> bool {
    for i in 0..types.count() {
        let t = types.objectAtIndex(i);
        // ...
```

Called 8-10 times per clipboard poll cycle, each iterating the full array.

**Desired behavior:**

Collect types into a `HashSet<String>` once, then check membership:

```rust
let type_set: HashSet<String> = (0..types.count())
    .map(|i| types.objectAtIndex(i).to_string())
    .collect();
// Then: type_set.contains("public.png")
```

**Acceptance criteria:**
- [ ] Types collected once per poll cycle
- [ ] Lookup is O(1) per check
- [ ] No behavioral change

**Risk:** Low.

---

### 3.15 Finding #23 — `indices_to_ranges` Clones and Sorts Input Unnecessarily

**Severity:** MINOR

**File:** `crates/photoncast-core/src/ui/result_item.rs` (lines 319–328)

**Current behavior:**

```rust
fn indices_to_ranges(indices: &[usize]) -> Vec<Range<usize>> {
    if indices.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut indices = indices.to_vec();  // ← unnecessary clone
    indices.sort_unstable();              // ← already sorted from nucleo
    indices.dedup();
    // ...
```

Indices from nucleo matcher are already sorted and deduplicated.

**Desired behavior:**

Remove the clone, sort, and dedup. Add a debug assertion:

```rust
fn indices_to_ranges(indices: &[usize]) -> Vec<Range<usize>> {
    debug_assert!(indices.windows(2).all(|w| w[0] < w[1]), "indices must be sorted and unique");
    // ... directly iterate indices without cloning
```

**Acceptance criteria:**
- [ ] No clone/sort/dedup of indices
- [ ] Debug assertion catches unsorted input in tests
- [ ] All existing tests pass

**Risk:** Low.

---

### 3.16 Finding #24 — Spotlight Cache Key Uses Heavy String Concatenation

**Severity:** MINOR

**File:** `crates/photoncast-core/src/search/spotlight/service.rs` (lines 400–413)

**Current behavior:**

```rust
fn make_cache_key(&self, query: &str, options: &SpotlightSearchOptions) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        query,
        options.max_results,
        options.apply_exclusions,
        options.sort_by_recency,
        options.primary_scopes.iter()
            .chain(options.secondary_scopes.iter())
            // ...
    )
}
```

Heavy string formatting with scope path concatenation on every search call.

**Desired behavior:**

Use a hash-based key:

```rust
fn make_cache_key(&self, query: &str, options: &SpotlightSearchOptions) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    query.hash(&mut hasher);
    options.max_results.hash(&mut hasher);
    options.apply_exclusions.hash(&mut hasher);
    options.sort_by_recency.hash(&mut hasher);
    for scope in options.primary_scopes.iter().chain(options.secondary_scopes.iter()) {
        scope.hash(&mut hasher);
    }
    hasher.finish()
}
```

**Acceptance criteria:**
- [ ] Cache key is a `u64` hash instead of a formatted `String`
- [ ] Cache hit/miss behavior is unchanged
- [ ] No hash collisions in existing test suite

**Risk:** Low — hash collisions are theoretically possible but extremely unlikely.

---

## 4. Work Stream 3: Structural Refactoring

### 4.1 Overview & Motivation

The codebase has several large files and structs that violate the Single Responsibility Principle. The largest offenders are `launcher.rs` (7632 lines), `file_search_view.rs` (2146 lines), and `main.rs` (1857 lines with a ~400-line `main()` function). Refactoring these improves maintainability, testability, and compile times.

### 4.2 Finding #25 — `main()` is ~400 Lines

**Severity:** CRITICAL

**File:** `crates/photoncast/src/main.rs` (lines 135–510+)

**Current behavior:**

The `main()` function contains:
- Logging initialization
- Config loading
- Extension discovery and loading
- Clipboard monitor setup
- Hotkey registration
- Window creation
- A ~300-line event loop with nested match arms

**Desired behavior:**

Extract into focused functions/structs:

```rust
fn main() {
    let config = init_logging_and_config();
    let app_state = AppState::init(config);
    let event_loop = EventLoop::new(app_state);
    event_loop.run();
}

struct AppState { /* shared state */ }

struct EventLoop {
    state: AppState,
    window_handles: WindowHandles,
}

impl EventLoop {
    fn handle_event(&mut self, event: AppEvent, cx: &mut AppContext) {
        match event {
            AppEvent::ToggleLauncher => self.handle_toggle_launcher(cx),
            AppEvent::OpenPreferences => self.handle_open_preferences(cx),
            // ...
        }
    }
}
```

**Acceptance criteria:**
- [ ] `main()` is < 50 lines
- [ ] Event handling extracted to `EventLoop` or similar struct
- [ ] Window management extracted to `WindowManager`
- [ ] Initialization logic extracted to focused functions
- [ ] No behavioral changes

**Risk:** Medium — large refactor but mostly mechanical.

---

### 4.3 Finding #26 — LauncherWindow Has 50+ Fields (SRP Violation)

**Severity:** CRITICAL

**File:** `crates/photoncast/src/launcher.rs` (lines 120–260)

**Current behavior:**

`LauncherWindow` struct contains 50+ fields mixing:
- Search state (query, cursor, results, selected_index)
- Animation state (animation_state, animation_start, selection_animation_start)
- Calculator state (calculator_command, calculator_runtime, calculator_result)
- Calendar state (next_meeting, calendar_all_events)
- App management (app_manager, uninstall_preview, auto_quit_manager)
- Toast notifications (toast_message, toast_shown_at)
- Window management (previous_frontmost_app, previous_frontmost_window_title)
- File search (file_search_view, file_search_loading)
- Extension views (extension_view, extension_view_id)

**Desired behavior:**

Extract into composable sub-structs:

```rust
struct LauncherWindow {
    search: SearchState,
    animation: AnimationState,
    calculator: CalculatorState,
    calendar: CalendarState,
    app_management: AppManagementState,
    ui: UIState,
    services: LauncherServices,
}
```

**Acceptance criteria:**
- [ ] Each sub-struct has < 10 fields
- [ ] Methods grouped by concern into impl blocks or traits
- [ ] No behavioral changes
- [ ] All existing tests pass

**Risk:** High — touches the core UI struct, many methods reference multiple fields. Must be done incrementally.

---

### 4.4 Finding #27 — `launcher.rs` is 7632 Lines

**Severity:** MAJOR

**File:** `crates/photoncast/src/launcher.rs`

**Current behavior:** Single monolithic file with all launcher logic.

**Desired behavior:**

Split into a `launcher/` module directory:

```
src/launcher/
├── mod.rs              # LauncherWindow struct + Render impl
├── search.rs           # Search handling methods
├── calculator.rs       # Calculator integration
├── calendar.rs         # Calendar/meeting widget
├── app_management.rs   # App management (quit, uninstall, auto-quit)
├── render.rs           # Render helper methods
├── actions_menu.rs     # Actions menu (Cmd+K)
├── suggestions.rs      # Suggestions rendering
├── icons.rs            # Icon caching logic
├── colors.rs           # LauncherColors
└── window.rs           # Window management helpers
```

**Acceptance criteria:**
- [ ] No file exceeds 1500 lines
- [ ] Public API of `launcher` module unchanged
- [ ] All existing tests pass
- [ ] Compile time not increased

**Risk:** Medium — large file split but mostly mechanical.

---

### 4.5 Finding #28 — `file_search_view.rs` is 2146 Lines

**Severity:** MAJOR

**File:** `crates/photoncast/src/file_search_view.rs`

**Desired behavior:**

Split into sub-modules:

```
src/file_search/
├── mod.rs          # FileSearchView struct + core logic
├── filter.rs       # Filter bar and file type filtering
├── browsing.rs     # Directory browsing state machine
├── render.rs       # Render implementations
├── helpers.rs      # Date formatting, file type helpers
└── colors.rs       # FileSearchColors (merge with LauncherColors per Finding #33)
```

**Acceptance criteria:**
- [ ] No file exceeds 800 lines
- [ ] Public API unchanged
- [ ] All existing tests pass

**Risk:** Medium.

---

### 4.6 Finding #29 — Duplicated Icon Caching Logic

**Severity:** MAJOR

**File:** `crates/photoncast/src/launcher.rs` (lines 940–1105)

**Current behavior:**

Four functions independently compute the cache directory and path hash:

- `get_app_icon_path_static()` (line 940) — computes cache dir, hash, extracts icon
- `clear_cached_icon()` (line 1067) — computes cache dir, hash, removes file
- `get_cached_icon_path()` (line 1096) — computes cache dir, hash, checks existence
- `get_app_icon_path()` (line 1175) — computes cache dir, hash, extracts icon

Each independently calls `ProjectDirs::from(...)`, `DefaultHasher`, and constructs the cache path.

**Desired behavior:**

Extract a shared helper:

```rust
struct IconCachePaths;

impl IconCachePaths {
    fn cache_dir() -> PathBuf {
        directories::ProjectDirs::from("", "", "PhotonCast")
            .map_or_else(
                || dirs::home_dir().unwrap_or_default().join("Library/Caches/PhotonCast/icons"),
                |dirs| dirs.cache_dir().join("icons"),
            )
    }

    fn cached_icon_path(app_path: &Path) -> PathBuf {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        app_path.hash(&mut hasher);
        Self::cache_dir().join(format!("{:x}.png", hasher.finish()))
    }
}
```

**Acceptance criteria:**
- [ ] Single `IconCachePaths` helper used by all four functions
- [ ] No duplicated cache dir computation
- [ ] No behavioral changes

**Risk:** Low.

---

### 4.7 Finding #30 — Duplicated Window Opening Boilerplate

**Severity:** MAJOR

**File:** `crates/photoncast/src/main.rs` (lines 1200–1750)

**Current behavior:**

8 `open_*_window()` functions (`open_launcher_window`, `open_clipboard_window`, `open_quicklinks_window`, `open_preferences_window`, `open_create_quicklink_window`, `open_argument_input_window`, `open_manage_quicklinks_window`, `open_timer_window`) each contain ~40 lines of identical display-bounds centering code.

**Desired behavior:**

Create a shared `open_window_centered()` helper:

```rust
fn open_window_centered<V: Render>(
    cx: &mut AppContext,
    width: Pixels,
    height: Pixels,
    build: impl FnOnce(&mut WindowContext) -> View<V>,
) -> Option<WindowHandle<V>> {
    let display = cx.primary_display()?;
    let bounds = display.bounds();
    let center_x = bounds.origin.x + (bounds.size.width - width) / 2.0;
    let center_y = bounds.origin.y + bounds.size.height * LAUNCHER_TOP_OFFSET_PERCENT;
    // ... common window options
    cx.open_window(options, build)
}
```

**Acceptance criteria:**
- [ ] Shared `open_window_centered()` helper used by all window-opening functions
- [ ] Each `open_*_window()` function reduced to < 15 lines
- [ ] No behavioral changes

**Risk:** Low.

---

### 4.8 Finding #31 — 44 Blanket Clippy `#![allow(...)]` Directives

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/lib.rs` (lines 18–64)

**Current behavior:**

44 crate-wide `#![allow(...)]` directives suppress warnings globally:

```rust
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::unused_async)]
// ... 41 more
#![allow(dead_code)]
```

This masks legitimate warnings that could catch real bugs.

**Desired behavior:**

1. Remove all blanket `#![allow(...)]` from `lib.rs`
2. Run `cargo clippy` and address each warning:
   - Fix the code where appropriate
   - Add targeted `#[allow(...)]` with a comment on specific items that genuinely need suppression
3. Keep a minimal set of crate-wide allows only for stylistic preferences that apply everywhere

**Acceptance criteria:**
- [ ] `lib.rs` has < 5 crate-wide `#![allow(...)]` directives
- [ ] All clippy warnings either fixed or locally suppressed with comments
- [ ] `cargo clippy` passes cleanly
- [ ] No behavioral changes

**Risk:** Medium — may surface many warnings to fix, but each is individually low-risk.

---

### 4.9 Finding #32 — Duplicated Frecency Query Logic

**Severity:** MAJOR

**File:** `crates/photoncast-core/src/storage/usage.rs` (lines 153–260)

**Current behavior:**

`get_command_frecency()` (line 153), `get_command_frecency_async()` (line 177), `get_file_frecency()` (line 228), and `get_file_frecency_async()` (line 252) contain nearly identical logic differing only in table/column names:

```rust
// get_command_frecency
conn.query_row(
    "SELECT use_count, last_used_at FROM command_usage WHERE command_id = ?1",
    [command_id],
    |row| Ok((row.get(0)?, row.get(1)?)),
)

// get_file_frecency
conn.query_row(
    "SELECT use_count, last_used_at FROM file_usage WHERE path = ?1",
    [path],
    |row| Ok((row.get(0)?, row.get(1)?)),
)
```

**Desired behavior:**

Extract a generic helper:

```rust
fn get_frecency_from_table(
    conn: &Connection,
    table: &str,
    key_column: &str,
    key_value: &str,
) -> Result<FrecencyScore> {
    let query = format!(
        "SELECT use_count, last_used_at FROM {} WHERE {} = ?1",
        table, key_column
    );
    // ... shared logic
}
```

**Acceptance criteria:**
- [ ] Single generic helper used by all 4 frecency functions
- [ ] No behavioral changes
- [ ] SQL injection safe (table/column names are compile-time constants, not user input)

**Risk:** Low.

---

### 4.10 Finding #33 — Duplicated Color Structs

**Severity:** MAJOR

**File:** `crates/photoncast/src/launcher.rs` (lines 62–106), `crates/photoncast/src/file_search_view.rs` (lines 218–250)

**Current behavior:**

`LauncherColors` has 17 fields and `FileSearchColors` has 10 fields, with significant overlap:

```rust
// LauncherColors (launcher.rs:63)
struct LauncherColors {
    background: Hsla, text: Hsla, text_muted: Hsla, text_placeholder: Hsla,
    surface: Hsla, surface_hover: Hsla, surface_elevated: Hsla, border: Hsla,
    accent: Hsla, accent_hover: Hsla, selection: Hsla, success: Hsla,
    warning: Hsla, error: Hsla, overlay: Hsla,
}

// FileSearchColors (file_search_view.rs:218)
pub struct FileSearchColors {
    pub background: Hsla, pub text: Hsla, pub text_muted: Hsla, pub text_placeholder: Hsla,
    pub surface: Hsla, pub surface_hover: Hsla, pub surface_elevated: Hsla, pub border: Hsla,
    pub accent: Hsla, pub selection: Hsla,
}
```

**Desired behavior:**

Create a shared `ThemeColorSet` in a common location:

```rust
// src/theme_colors.rs or similar
pub struct ThemeColorSet {
    pub background: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_placeholder: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub surface_elevated: Hsla,
    pub border: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub selection: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub overlay: Hsla,
}
```

**Acceptance criteria:**
- [ ] Single `ThemeColorSet` struct used by both launcher and file search
- [ ] `from_theme()` implemented once
- [ ] No visual changes

**Risk:** Low.

---

## 5. Work Stream 4: Code Cleanup

### 5.1 Overview & Motivation

Minor code quality issues that improve readability and maintainability. These can be tackled independently and in any order.

### 5.2 Finding #34 — `ClipboardState.monitor` Marked `#[allow(dead_code)]`

**Severity:** MINOR

**File:** `crates/photoncast/src/main.rs` (lines 79–83)

**Current behavior:**

```rust
struct ClipboardState {
    storage: ClipboardStorage,
    config: ClipboardConfig,
    #[allow(dead_code)]
    monitor: Option<Arc<ClipboardMonitor>>,
}
```

**Desired behavior:**

Replace `#[allow(dead_code)]` with an explanatory comment:

```rust
struct ClipboardState {
    storage: ClipboardStorage,
    config: ClipboardConfig,
    /// Held to keep the background clipboard monitoring task alive.
    /// The monitor runs its own polling loop; dropping it stops monitoring.
    _monitor: Option<Arc<ClipboardMonitor>>,
}
```

Using the `_` prefix convention makes the intent explicit without suppressing warnings.

**Acceptance criteria:**
- [ ] `#[allow(dead_code)]` removed
- [ ] Field renamed to `_monitor` with doc comment
- [ ] No behavioral change

**Risk:** None.

---

### 5.3 Finding #35 — Duplicated Extension Lists

**Severity:** MINOR

**File:** `crates/photoncast/src/file_search_helper.rs` (line 404)

**Current behavior:**

File type extension lists (e.g., `INTERESTING_EXTENSIONS`) are defined in `file_search_helper.rs`. If similar lists exist in `file_search_view.rs`, they should be consolidated.

**Desired behavior:**

Define all file type extension constants in one place (e.g., `constants.rs` or a dedicated `file_types.rs` module).

**Acceptance criteria:**
- [ ] File type extension lists defined in one location
- [ ] All consumers import from the single source
- [ ] No behavioral change

**Risk:** None.

---

### 5.4 Finding #36 — `TimerError` Missing `From` Impls

**Severity:** MINOR

**File:** `crates/photoncast-timer/src/lib.rs` (line 47, re-exported)

**Current behavior:**

`TimerError` requires manual string conversion at every call site (e.g., `TimerError::Parse(format!(...))`) unlike other error types in the project that use `From` impls or `thiserror` derive.

**Desired behavior:**

Add `From` impls or use `thiserror` `#[from]` for common error conversions:

```rust
#[derive(Debug, Error)]
pub enum TimerError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    // etc.
}
```

**Acceptance criteria:**
- [ ] `TimerError` uses `#[from]` for standard error types
- [ ] Call sites simplified where possible
- [ ] No behavioral change

**Risk:** None.

---

### 5.5 Finding #37 — Redefined Constants

**Severity:** MINOR

**Files:**
- `crates/photoncast/src/constants.rs` (lines 39, 42)
- `crates/photoncast/src/launcher.rs` (line 109)
- `crates/photoncast/src/extension_views/mod.rs` (line 114)

**Current behavior:**

```rust
// constants.rs:39
pub const SEARCH_BAR_HEIGHT: Pixels = px(48.0);
// constants.rs:42
pub const LIST_ITEM_HEIGHT: Pixels = px(56.0);

// launcher.rs:109
const SEARCH_BAR_HEIGHT: Pixels = px(48.0);  // ← redefined

// extension_views/mod.rs:114
pub const SEARCH_BAR_HEIGHT: Pixels = gpui::px(44.0);  // ← different value!
```

Note: The extension views search bar is intentionally 44px (smaller), but the name collision is confusing.

**Desired behavior:**

1. Use `constants::SEARCH_BAR_HEIGHT` in `launcher.rs` instead of redefining
2. Rename the extension views constant to `EXTENSION_SEARCH_BAR_HEIGHT` to avoid confusion
3. Import from `constants.rs` wherever possible

**Acceptance criteria:**
- [ ] No duplicate constant definitions with the same name
- [ ] Extension views constant renamed to be distinct
- [ ] All consumers use the canonical source

**Risk:** None.

---

### 5.6 Finding #38 — Magic Numbers

**Severity:** MINOR

**Files:** Various

**Current behavior:**

Hardcoded values scattered throughout the codebase:
- Overlay alpha `0.6` in `LauncherColors::from_theme()` (launcher.rs:103)
- Icon sizes (64×64 in `sips` calls)
- Animation durations
- Various pixel values

**Desired behavior:**

Extract to named constants in `constants.rs`:

```rust
pub const OVERLAY_ALPHA: f32 = 0.6;
pub const ICON_CACHE_SIZE: u32 = 64;
// etc.
```

**Acceptance criteria:**
- [ ] All hardcoded numeric values in rendering/animation code replaced with named constants
- [ ] Constants defined in `constants.rs` or relevant module
- [ ] No behavioral change

**Risk:** None.

---

### 5.7 Finding #39 — `resize_window_height` / `get_window_height` Marked Dead Code

**Severity:** MINOR

**File:** `crates/photoncast/src/platform.rs` (lines 32–65)

**Current behavior:**

```rust
#[allow(dead_code)]
pub fn resize_window_height(new_height: f64) { /* ... */ }

#[allow(dead_code)]
pub fn get_window_height() -> Option<f64> { /* ... */ }
```

Both functions are fully implemented but unused, with `#[allow(dead_code)]` suppressing warnings.

**Desired behavior:**

Either:
- **Remove** if no longer needed (a `resize_window` function exists that handles both dimensions)
- **Use** if they serve a purpose
- **Document** with `// TODO:` explaining why they're kept

**Acceptance criteria:**
- [ ] Functions either removed, used, or documented with rationale
- [ ] No `#[allow(dead_code)]` without explanation

**Risk:** None.

---

## 6. Cross-Cutting Concerns

### 6.1 Dependencies Between Work Streams

| Dependency | Details |
|---|---|
| WS2 Finding #16 depends on WS3 Finding #26 | `LauncherWindow` sub-struct split is the same work |
| WS3 Finding #27 depends on WS3 Finding #26 | File split should happen after struct split |
| WS3 Finding #33 should precede WS3 Finding #27/28 | Shared colors struct should exist before file split |
| WS1 Findings #3-5 should be done together | All in `actions.rs`, shared validation logic |
| WS4 can be done independently | No dependencies on other work streams |

### 6.2 Recommended Execution Order

1. **Phase 1 — Quick wins** (1-2 days): WS4 (all), WS2 #20 (dead code), WS2 #23 (indices_to_ranges), WS1 #7-8 (timestamp, expand_tilde)
2. **Phase 2 — Security** (2-3 days): WS1 #1-5 (extension security + action validation)
3. **Phase 3 — Performance** (3-4 days): WS2 #10-12 (critical blocking I/O), WS2 #13-14 (ranking allocations), WS2 #17-19 (cloning/dedup)
4. **Phase 4 — Structural** (4-5 days): WS3 #33 (shared colors), WS3 #26 (LauncherWindow split), WS3 #27-28 (file splits), WS3 #25 (main refactor), WS3 #29-32 (dedup)

### 6.3 Testing Strategy

| Work Stream | Testing Approach |
|---|---|
| Security Hardening | Unit tests for path traversal, URL validation, signature verification. Integration test with mock extension loading. |
| Performance | Benchmarks using `criterion` for search ranking, icon loading, and directory scanning. Before/after comparison. |
| Structural Refactoring | Existing tests must pass unchanged. No new behavior to test. |
| Code Cleanup | Compiler warnings (`cargo clippy`) should be clean. Existing tests must pass. |

**Global verification:**
- `cargo clippy --all-targets` passes cleanly
- `cargo test --all` passes
- Manual smoke test of launcher, file search, clipboard, and extension loading
