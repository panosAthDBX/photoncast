# Tasks List for PhotonCast Code Review Fixes

> Generated from code review findings. 39 findings organized into 4 phases with dependency ordering.

---

## Phase 1: Security Hardening

Priority: **Highest** — These tasks address vulnerabilities that could allow malicious extensions to compromise user systems.

---

### Task 1.1: Validate extension action paths and URLs before execution
- **Findings**: #3, #4
- **Files to modify**:
  - `crates/photoncast/src/extension_views/actions.rs`
- **Description**: The `execute_action()` function passes extension-supplied strings directly to `open::that()` and `std::process::Command` without validation. `ActionHandler::OpenUrl`, `ActionHandler::OpenFile`, and `ActionHandler::RevealInFinder` all trust extension input blindly.
- **Implementation Steps**:
  1. Create a `validate_url(url: &str) -> Result<(), ActionError>` function that:
     - Parses the URL with `url::Url` (or a lightweight check)
     - Rejects non-`http`/`https` schemes (e.g., `file://`, `javascript:`)
     - Optionally allowlists specific URL schemes if needed
  2. Create a `validate_path(path: &str) -> Result<PathBuf, ActionError>` function that:
     - Canonicalizes the path via `std::fs::canonicalize()` or `std::path::Path::canonicalize()`
     - Verifies the path exists on the filesystem
     - Rejects paths containing `..` traversal segments before canonicalization
  3. In `ActionHandler::OpenUrl(url)` — call `validate_url()` before `open::that()`
  4. In `ActionHandler::OpenFile(path)` — call `validate_path()` before `open::that()`
  5. In `ActionHandler::RevealInFinder(path)` — call `validate_path()` before `Command::new("open").args(["-R", &path])`
  6. In `ActionHandler::QuickLook(path)` — call `validate_path()` before `Command::new("qlmanage")`
  7. Log warnings for rejected actions with extension context
- **Testing Requirements**:
  - Unit tests for `validate_url` with valid/invalid schemes
  - Unit tests for `validate_path` with traversal attempts, non-existent paths
  - Integration test that malicious paths are rejected
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 1.2: Prevent path traversal in extension dylib resolution
- **Findings**: #2
- **Files to modify**:
  - `crates/photoncast-core/src/extensions/manager.rs` (function `resolve_entry_path` at bottom of file)
  - `crates/photoncast-core/src/extensions/manifest.rs` (if manifest parsing needs validation)
- **Description**: The `resolve_entry_path()` function joins `manifest.entry.path` to the base directory without checking the result stays within the extension directory. A manifest with `entry.path = "../../malicious.dylib"` would resolve outside the extension dir.
- **Implementation Steps**:
  1. In `resolve_entry_path()`, after computing the joined path, canonicalize it
  2. Canonicalize the `base_dir` as well
  3. Assert `canonical_path.starts_with(&canonical_base_dir)`
  4. Return an error (not panic) if the path escapes the extension directory
  5. Change return type from `PathBuf` to `Result<PathBuf, ExtensionManagerError>` and propagate errors in `load_and_activate()` and `reload_extension()`
  6. Add a new `ExtensionManagerError::PathTraversal` variant
- **Testing Requirements**:
  - Unit test: normal relative path resolves correctly
  - Unit test: `../../malicious.dylib` is rejected
  - Unit test: symlink pointing outside dir is rejected (if canonicalize is used)
- **Complexity**: Small
- **Dependencies**: None

---

### Task 1.3: Add code signing verification for extension dylibs
- **Findings**: #1
- **Files to modify**:
  - `crates/photoncast-core/src/extensions/manager.rs` (in `load_and_activate()` and `reload_extension()`)
  - New file: `crates/photoncast-core/src/extensions/signing.rs`
- **Description**: Extensions are loaded as native code via `abi_stable::lib_header_from_path` without verifying code signatures. A malicious dylib in the extensions directory executes with full app privileges.
- **Implementation Steps**:
  1. Create `crates/photoncast-core/src/extensions/signing.rs` module
  2. Implement `verify_code_signature(path: &Path) -> Result<(), SigningError>` using macOS `codesign --verify` via `Command`
  3. Alternatively, implement an allowlist approach: maintain a `known_extensions.toml` with SHA-256 hashes of trusted dylibs
  4. In `load_and_activate()`, call verification before `ExtensionLoader::load(&entry_path)`
  5. In `reload_extension()`, call verification before `ExtensionLoader::load(&load_path)`
  6. In dev mode (`self.dev_mode`), skip verification but log a warning
  7. Add `mod signing;` to `crates/photoncast-core/src/extensions/mod.rs`
- **Testing Requirements**:
  - Unit test: valid signed dylib passes
  - Unit test: unsigned dylib is rejected (unless dev mode)
  - Unit test: dev mode bypasses check with warning
- **Complexity**: Large
- **Dependencies**: Task 1.2 (path validation should happen first)

---

### Task 1.4: Add clipboard write logging for extension actions
- **Findings**: #5
- **Files to modify**:
  - `crates/photoncast/src/extension_views/actions.rs`
- **Description**: `ActionHandler::CopyToClipboard` writes extension-supplied content to the system clipboard without any logging. While user confirmation may be impractical, logging provides an audit trail.
- **Implementation Steps**:
  1. In the `CopyToClipboard` match arm (~line 51), add a `tracing::info!` log entry with:
     - The extension ID (may need to thread it through to `execute_action`)
     - A truncated preview of the content (first 100 chars)
     - Content length
  2. Consider adding the extension ID as a parameter to `execute_action()` for better audit logging across all action types
- **Testing Requirements**:
  - Verify log output contains expected fields
- **Complexity**: Small
- **Dependencies**: None

---

### Task 1.5: Audit unsafe Send+Sync implementations
- **Findings**: #6
- **Files to modify**:
  - `crates/photoncast-core/src/extensions/storage.rs` (lines 34-35)
  - `crates/photoncast-core/src/extensions/host.rs` (lines 41-42)
- **Description**: `unsafe impl Send/Sync` for `ExtensionStorageImpl` and `ExtensionHostServices`. These may be unsound if fields hold raw pointers from dynamically loaded libraries that don't satisfy thread-safety guarantees.
- **Implementation Steps**:
  1. Audit all fields of `ExtensionStorageImpl` — verify each field type is `Send + Sync`
  2. Audit all fields of `ExtensionHostServices` — verify each field type is `Send + Sync`
  3. If fields are inherently `Send+Sync`, add `// SAFETY: ...` comments documenting why
  4. If any field is not thread-safe, wrap it in appropriate synchronization (e.g., `Mutex`, `Arc<Mutex<>>`)
  5. If wrapping is sufficient, remove the `unsafe impl` and let the compiler derive the traits
- **Testing Requirements**:
  - Compile-time verification (if unsafe impls removed, compiler checks)
  - Document safety invariants
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 1.6: Fix Y2038 timestamp truncation in schema versioning
- **Findings**: #7
- **Files to modify**:
  - `crates/photoncast-core/src/storage/database.rs` (function `record_version` around line 118-127)
- **Description**: `record_version()` stores `now as i32` for the `applied_at` column. `i32` timestamps overflow on January 19, 2038.
- **Implementation Steps**:
  1. In `record_version()` (line ~124), change `&(now as i32)` to `&now` (which is already `i64` from `Utc::now().timestamp()`)
  2. Verify the `schema_version` table DDL uses `INTEGER` (SQLite `INTEGER` supports 64-bit)
  3. Check if any other `as i32` timestamp casts exist in `database.rs` and fix them
- **Testing Requirements**:
  - Unit test inserting a timestamp > 2^31 and reading it back
- **Complexity**: Small
- **Dependencies**: None

---

### Task 1.7: Fix incomplete `expand_tilde` function
- **Findings**: #8
- **Files to modify**:
  - `crates/photoncast-core/src/utils/paths.rs` (function `expand_tilde` at line 54)
- **Description**: `expand_tilde` only handles `~/...` prefix. A bare `~` (without trailing `/`) would attempt `h.join(&path[2..])` which panics or produces wrong output since `path[2..]` is out of bounds for a 1-char string.
- **Implementation Steps**:
  1. Change the check from `path.starts_with('~')` with indexing `&path[2..]` to properly handle:
     - `~` alone → return home directory
     - `~/something` → return `home_dir/something`
     - Other `~user` forms → leave as-is (or return as-is)
  2. Suggested implementation:
     ```rust
     if path == "~" {
         dirs::home_dir().unwrap_or_else(|| PathBuf::from(path))
     } else if let Some(rest) = path.strip_prefix("~/") {
         dirs::home_dir()
             .map(|h| h.join(rest))
             .unwrap_or_else(|| PathBuf::from(path))
     } else {
         PathBuf::from(path)
     }
     ```
- **Testing Requirements**:
  - Unit test: `expand_tilde("~")` returns home dir
  - Unit test: `expand_tilde("~/foo")` returns `home/foo`
  - Unit test: `expand_tilde("/absolute")` returns unchanged
  - Unit test: `expand_tilde("relative")` returns unchanged
- **Complexity**: Small
- **Dependencies**: None

---

### Task 1.8: Verify Phase 1 — Security
- **Description**: Run full test suite, clippy, and manual testing to verify all security fixes.
- **Implementation Steps**:
  1. Run `cargo clippy --workspace -- -D warnings` (or project-specific lint config)
  2. Run `cargo test --workspace`
  3. Manually test extension loading with a test extension
  4. Verify action execution logs appear for clipboard writes
  5. Verify path traversal is rejected in extension manifest loading
- **Complexity**: Small
- **Dependencies**: Tasks 1.1–1.7

---

## Phase 2: Critical Performance Fixes

Priority: **High** — These tasks address UI freezes, startup latency, and hot-path allocations.

---

### Task 2.1: Make screenshot thumbnail generation async
- **Findings**: #10
- **Files to modify**:
  - `crates/photoncast-ext-screenshots/src/lib.rs` (thumbnail generation around lines 82-100)
- **Description**: `image::open(path)` and `thumbnail.save()` are synchronous I/O on the command handler thread. For folders with many large images, this freezes the UI for seconds.
- **Implementation Steps**:
  1. Move thumbnail generation into a background thread or async task
  2. Use `tokio::task::spawn_blocking` or `std::thread::spawn` to offload `image::open()` and `thumbnail.save()`
  3. Return a placeholder/loading state while thumbnails generate
  4. Cache generated thumbnails (already done in `get_or_create_thumbnail`) — ensure the cache check is fast and synchronous
  5. Consider batch-processing thumbnails with a bounded concurrency (e.g., 4 at a time)
- **Testing Requirements**:
  - Verify UI remains responsive during thumbnail generation
  - Verify thumbnails are generated correctly
  - Verify cache hits bypass I/O
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 2.2: Cache directory listing in screenshots extension
- **Findings**: #11
- **Files to modify**:
  - `crates/photoncast-ext-screenshots/src/lib.rs` (function `scan_screenshots` around lines 212-237)
- **Description**: `scan_screenshots` does a full `read_dir` + metadata + sort on every command invocation. No caching of the directory listing.
- **Implementation Steps**:
  1. Add a cached directory listing field to the extension struct (or a module-level cache)
  2. Store the list of `Screenshot` structs with a timestamp of last scan
  3. On subsequent calls, check if the directory's modification time has changed before rescanning
  4. Use `std::fs::metadata(dir).modified()` to detect directory changes
  5. Invalidate cache when a delete/rename action is performed
- **Testing Requirements**:
  - Verify first call performs full scan
  - Verify subsequent calls use cache when directory unchanged
  - Verify cache invalidates when directory changes
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 2.3: Make icon extraction async with batching
- **Findings**: #12
- **Files to modify**:
  - `crates/photoncast/src/launcher.rs` (functions `get_app_icon_path_static` around line 940, `sips` calls around lines 999-1050)
- **Description**: `std::process::Command::new("sips")...output()` is synchronous and spawned per-app during indexing (~200 spawns at startup), blocking the indexing thread.
- **Implementation Steps**:
  1. Replace synchronous `Command::new("sips")...output()` with `tokio::process::Command` or spawn on a background thread pool
  2. Batch icon extraction: collect all apps needing icon extraction, then process them concurrently with bounded parallelism (e.g., `futures::stream::buffer_unordered(8)`)
  3. Return cached icons immediately, queue uncached ones for background extraction
  4. Notify UI when icons become available (via channel or callback)
- **Testing Requirements**:
  - Verify startup time improvement with timing logs
  - Verify icons still display correctly after async extraction
  - Verify cache hits are instant
- **Complexity**: Large
- **Dependencies**: None

---

### Task 2.4: Pre-compute lowercased strings in ranking hot path
- **Findings**: #13, #14
- **Files to modify**:
  - `crates/photoncast-core/src/search/ranking.rs` (lines ~166-175 and ~227)
- **Description**: `to_lowercase()` is called per-result in the ranking loop (2 allocations per result) and again inside the sort comparator (O(n log n) allocations).
- **Implementation Steps**:
  1. Before the ranking loop, pre-compute `query_lower = query.to_lowercase()`
  2. Create a vec of `(index, lowercased_title)` tuples before ranking
  3. In the tiebreaker sort (line ~227), sort using the pre-computed lowercased titles instead of calling `to_lowercase()` inside the comparator
  4. Consider using a struct like `RankedResult { result: SearchResult, title_lower: String, score: f64 }` to carry the pre-computed data through ranking and sorting
- **Testing Requirements**:
  - Existing ranking tests still pass
  - Benchmark with 500+ results shows improvement
- **Complexity**: Small
- **Dependencies**: None

---

### Task 2.5: Eliminate per-keystroke cloning in extension list view
- **Findings**: #17
- **Files to modify**:
  - `crates/photoncast/src/extension_views/list_view.rs` (function `apply_search_filter` around line 207)
- **Description**: `apply_search_filter()` clones every `FlatListItem` on every keystroke when filtering. `filtered_items` should store indices into `flat_items` instead of cloned items.
- **Implementation Steps**:
  1. Change `filtered_items: Vec<FlatListItem>` to `filtered_items: Vec<usize>` (indices into `flat_items`)
  2. Update `apply_search_filter()` to collect indices instead of cloning items
  3. Update all access sites that read `self.filtered_items[i]` to use `self.flat_items[self.filtered_items[i]]`
  4. Update `selected_index` bounds checking accordingly
  5. Ensure the render methods dereference through the index correctly
- **Testing Requirements**:
  - Extension list view renders correctly
  - Filtering works as before
  - Performance improvement measurable in profiling
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 2.6: Use `Arc<SearchResult>` for flat_results in ResultsList
- **Findings**: #18
- **Files to modify**:
  - `crates/photoncast-core/src/ui/results_list.rs` (function `set_results` at line ~71)
  - `crates/photoncast-core/src/search/mod.rs` (if `SearchResults` needs `Arc` support)
- **Description**: `set_results()` clones every `SearchResult` via `results.iter().cloned().collect()` on every search update.
- **Implementation Steps**:
  1. Change `flat_results: Vec<SearchResult>` to `flat_results: Vec<Arc<SearchResult>>` in the `ResultsList` struct
  2. Have `SearchResults` store `Arc<SearchResult>` internally, or wrap during `set_results()`
  3. Update `set_results()` to use `Arc::clone()` instead of deep cloning
  4. Update all consumers of `flat_results` to work with `Arc<SearchResult>`
- **Testing Requirements**:
  - All existing search result display tests pass
  - Memory usage reduced for large result sets
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 2.7: Replace quadratic deduplication with HashSet in prefetch
- **Findings**: #19
- **Files to modify**:
  - `crates/photoncast-core/src/search/spotlight/prefetch.rs` (around lines 271-295)
- **Description**: Deduplication loop uses `files.iter().any(|f| f.path == result.path)` which is O(n*m). Use a `HashSet<PathBuf>` for O(1) lookups.
- **Implementation Steps**:
  1. Before the merge loop, create `let mut seen_paths: HashSet<PathBuf> = files.lock().iter().map(|f| f.path.clone()).collect();`
  2. Replace `if !files.iter().any(|f| f.path == result.path)` with `if seen_paths.insert(result.path.clone())`
  3. This changes complexity from O(n*m) to O(n+m)
- **Testing Requirements**:
  - Prefetch results are still deduplicated correctly
  - No duplicate paths in merged results
- **Complexity**: Small
- **Dependencies**: None

---

### Task 2.8: Remove dead `_match_ranges` computation
- **Findings**: #20
- **Files to modify**:
  - `crates/photoncast-core/src/ui/result_item.rs` (around lines 206-215)
- **Description**: Code computes `_match_ranges` via `indices_to_ranges()` but the result is never used (prefixed with `_`).
- **Implementation Steps**:
  1. Remove the `_match_ranges` variable and the `indices_to_ranges()` call
  2. If `indices_to_ranges()` is only used here, consider removing it too (check for other callers first)
- **Testing Requirements**:
  - Verify no compilation errors
  - Verify match highlighting still works (it likely uses a different code path)
- **Complexity**: Small
- **Dependencies**: None

---

### Task 2.9: Use LRU crate for icon cache eviction
- **Findings**: #21
- **Files to modify**:
  - `crates/photoncast-core/src/indexer/icons.rs` (around lines 126-136)
- **Description**: Icon cache uses a manual O(n) LRU scan for eviction. The `lru` crate is already a dependency in the project.
- **Implementation Steps**:
  1. Replace the custom `HashMap + access_counter` implementation with `lru::LruCache`
  2. Initialize with the existing `max_size` capacity
  3. Replace manual `get_mut` + counter update with `lru::LruCache::get()`
  4. Remove the manual eviction loop — `LruCache` handles this automatically on `put()`
  5. Update the `insert()`, `get()`, `clear()` methods accordingly
- **Testing Requirements**:
  - Icon cache stores and retrieves icons correctly
  - Eviction occurs at capacity limit
  - LRU ordering is correct
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 2.10: Optimize clipboard type checking with HashSet
- **Findings**: #22
- **Files to modify**:
  - `crates/photoncast-clipboard/src/monitor.rs` (around lines 375-382)
- **Description**: `has_type` iterates the full type array per check, with 4-5 checks per poll cycle. Collecting types into a `HashSet` first would reduce this to O(1) per check.
- **Implementation Steps**:
  1. At the start of the poll/check function, collect available types into a `HashSet<String>` or `HashSet<&str>`
  2. Replace individual `has_type` calls (which iterate the array) with `HashSet::contains()`
  3. This reduces 4-5 full array scans to a single collection + O(1) lookups
- **Testing Requirements**:
  - Clipboard monitoring still detects all content types
  - No regressions in clipboard change detection
- **Complexity**: Small
- **Dependencies**: None

---

### Task 2.11: Skip unnecessary sort in `indices_to_ranges`
- **Findings**: #23
- **Files to modify**:
  - `crates/photoncast-core/src/ui/result_item.rs` (function `indices_to_ranges` around line 236)
- **Description**: `indices_to_ranges` clones and sorts the input indices, but nucleo already returns sorted indices.
- **Implementation Steps**:
  1. Remove the `.clone()` and `.sort()` calls
  2. Accept `&[u32]` (or `&[usize]`) directly and iterate without copying
  3. Add a debug assertion `debug_assert!(indices.windows(2).all(|w| w[0] <= w[1]))` to catch if the assumption ever breaks
- **Testing Requirements**:
  - Match highlighting still works correctly
  - Debug assertion doesn't fire in tests
- **Complexity**: Small
- **Dependencies**: Task 2.8 (if `indices_to_ranges` is removed entirely, skip this)

---

### Task 2.12: Optimize Spotlight cache key generation
- **Findings**: #24
- **Files to modify**:
  - `crates/photoncast-core/src/search/spotlight/service.rs` (function `make_cache_key` around line ~263-276)
- **Description**: Cache key uses heavy string concatenation. A hash-based key would be faster and use less memory.
- **Implementation Steps**:
  1. Replace string concatenation with hashing (e.g., `std::hash::DefaultHasher` or `ahash`)
  2. Hash the query string and relevant options fields
  3. Use `u64` as the cache key type instead of `String`
  4. Update the cache `HashMap` key type accordingly
- **Testing Requirements**:
  - Cache hits still work correctly
  - No hash collisions in test scenarios
- **Complexity**: Small
- **Dependencies**: None

---

### Task 2.13: Verify Phase 2 — Performance
- **Description**: Run full test suite and benchmark critical paths to verify performance improvements.
- **Implementation Steps**:
  1. Run `cargo clippy --workspace -- -D warnings`
  2. Run `cargo test --workspace`
  3. Profile search with 500+ results — verify no per-keystroke allocations in ranking
  4. Profile extension list view — verify no per-keystroke cloning
  5. Measure startup time — verify icon extraction doesn't block
  6. Test screenshots extension with large folders — verify no UI freeze
- **Complexity**: Medium
- **Dependencies**: Tasks 2.1–2.12

---

## Phase 3: Structural Refactoring

Priority: **Medium** — These tasks improve maintainability and reduce technical debt through architectural improvements.

---

### Task 3.1: Extract shared icon cache helper from LauncherWindow
- **Findings**: #29
- **Files to modify**:
  - `crates/photoncast/src/launcher.rs` (functions around lines 939-1096: `get_app_icon_path_static`, `clear_cached_icon`, `get_cached_icon_path`, `get_app_icon_path`)
  - New file: `crates/photoncast/src/icon_cache.rs`
- **Description**: Four functions independently compute cache directory and hash for icon paths. Extract a shared helper module.
- **Implementation Steps**:
  1. Create `crates/photoncast/src/icon_cache.rs`
  2. Extract a shared `IconCacheDir` struct with:
     - `fn cache_dir() -> PathBuf` — computes the icon cache directory once
     - `fn cache_path_for_app(app_path: &Path) -> PathBuf` — computes cache path from app path hash
     - `fn get_icon(app_path: &Path) -> Option<PathBuf>` — checks cache
     - `fn extract_icon(app_path: &Path) -> Option<PathBuf>` — extracts via sips
     - `fn clear_icon(app_path: &Path)` — removes cached icon
  3. Replace all 4 functions in `launcher.rs` with calls to the new module
  4. Add `mod icon_cache;` to the crate root
- **Testing Requirements**:
  - All icon display functionality works as before
  - Cache operations are correct
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 3.2: Extract shared `open_window_centered` helper
- **Findings**: #30
- **Files to modify**:
  - `crates/photoncast/src/main.rs` (functions at lines 1200-1686: `open_launcher_window`, `open_clipboard_window`, `open_quicklinks_window`, `open_preferences_window`, `open_create_quicklink_window`, `open_argument_input_window`, `open_manage_quicklinks_window`, `open_timer_window`)
  - New file or section: shared window opening utility
- **Description**: 8 `open_*_window()` functions contain ~40 lines of identical display-bounds centering code each.
- **Implementation Steps**:
  1. Create a shared `open_window_centered()` function that accepts:
     - Window size (width, height)
     - Window options (title, focus behavior, etc.)
     - A view builder closure
  2. Move the display-bounds calculation, centering logic, and window opening into this helper
  3. Refactor each `open_*_window()` to call `open_window_centered()` with its specific parameters
  4. This should reduce each function to ~5-10 lines
- **Testing Requirements**:
  - All windows open at correct positions
  - Window centering works on multi-monitor setups
- **Complexity**: Medium
- **Dependencies**: None

---

### Task 3.3: Extract duplicated frecency query logic
- **Findings**: #32
- **Files to modify**:
  - `crates/photoncast-core/src/storage/usage.rs` (functions at lines 153, 177, 228, 252)
- **Description**: `get_command_frecency()` and `get_file_frecency()` (sync + async) have identical logic differing only in table/column names. Same for `record_command_use` / `record_file_use`.
- **Implementation Steps**:
  1. Create a generic private helper:
     ```rust
     fn get_frecency_for(&self, table: &str, key_column: &str, key_value: &str) -> Result<FrecencyScore>
     ```
  2. Refactor `get_command_frecency()` and `get_file_frecency()` to call the helper with their respective table/column names
  3. Do the same for async variants using `spawn_blocking`
  4. Similarly, create a generic `record_use()` helper for the recording functions
- **Testing Requirements**:
  - All existing frecency tests pass
  - Both command and file frecency compute correctly
- **Complexity**: Small
- **Dependencies**: None

---

### Task 3.4: Create shared `ThemeColorSet` struct
- **Findings**: #33
- **Files to modify**:
  - `crates/photoncast/src/launcher.rs` (struct `LauncherColors` at line 63)
  - `crates/photoncast/src/file_search_view.rs` (struct `FileSearchColors` at line 218)
  - New file or shared location for `ThemeColorSet`
- **Description**: `LauncherColors` and `FileSearchColors` have identical fields and derivation. Extract a shared struct.
- **Implementation Steps**:
  1. Define a shared `ThemeColorSet` struct (in `crates/photoncast/src/theme_colors.rs` or `constants.rs`)
  2. Type-alias or replace `LauncherColors` and `FileSearchColors` with `ThemeColorSet`
  3. Update all construction sites and field accesses
- **Testing Requirements**:
  - Both launcher and file search views render with correct colors
- **Complexity**: Small
- **Dependencies**: None

---

### Task 3.5: Deduplicate constant definitions
- **Findings**: #37
- **Files to modify**:
  - `crates/photoncast/src/launcher.rs` (line 109: `const SEARCH_BAR_HEIGHT`)
  - `crates/photoncast/src/constants.rs` (lines 39, 42)
  - `crates/photoncast/src/extension_views/mod.rs` (line 114)
- **Description**: `SEARCH_BAR_HEIGHT` and `LIST_ITEM_HEIGHT` are defined in multiple places with potentially different values (48.0 vs 44.0 for search bar height).
- **Implementation Steps**:
  1. Decide on canonical values for each constant
  2. Define them once in `constants.rs`
  3. Remove duplicates from `launcher.rs` and `extension_views/mod.rs`
  4. Update all `use` imports to reference `constants::SEARCH_BAR_HEIGHT` etc.
  5. If extension views intentionally use different values, rename them (e.g., `EXTENSION_SEARCH_BAR_HEIGHT`)
- **Testing Requirements**:
  - All views render with correct dimensions
  - No compilation errors from missing constants
- **Complexity**: Small
- **Dependencies**: None

---

### Task 3.6: Deduplicate file type extension lists
- **Findings**: #35
- **Files to modify**:
  - `crates/photoncast/src/file_search_view.rs`
  - `crates/photoncast/src/file_search_helper.rs` (if it exists)
- **Description**: File type extension lists (e.g., image extensions, document extensions) are duplicated between files.
- **Implementation Steps**:
  1. Identify all duplicated extension lists
  2. Define them as `const` arrays in a shared location (e.g., `constants.rs` or a dedicated `file_types.rs`)
  3. Update both files to reference the shared constants
- **Testing Requirements**:
  - File type filtering works correctly in both views
- **Complexity**: Small
- **Dependencies**: None

---

### Task 3.7: Split `LauncherWindow` into sub-structs
- **Findings**: #16, #26
- **Files to modify**:
  - `crates/photoncast/src/launcher.rs` (struct `LauncherWindow` at line 119, 50+ fields)
- **Description**: `LauncherWindow` has 50+ fields mixing search state, animation, calculator, calendar, app management, toast notifications, uninstall preview, etc. This violates SRP and causes cache pressure.
- **Implementation Steps**:
  1. Group related fields into sub-structs:
     - `SearchState { query, cursor_position, selection_anchor, cursor_blink_epoch, results, base_results, core_results, ... }`
     - `AnimationState { animation_state, animation_start, selection_animation_start, hover_animation_starts, ... }`
     - `CalculatorState { calculator_command, calculator_runtime, calculator_result, calculator_generation }`
     - `FileSearchState { file_search_view, file_search_loading, file_search_pending_query, file_search_generation }`
     - `ExtensionViewState { extension_view, extension_view_id }`
     - `UninstallState { uninstall_preview, uninstall_files_selected_index }`
     - `MeetingState { next_meeting, meeting_selected, calendar_all_events }`
  2. Replace individual fields in `LauncherWindow` with these sub-structs
  3. Update all field accesses throughout `launcher.rs` (e.g., `self.query` → `self.search.query`)
  4. Hot-path fields (search state, selected_index) should be in the same sub-struct for cache locality
- **Testing Requirements**:
  - All launcher functionality works as before
  - No performance regression
- **Complexity**: XL (large file, many field accesses to update)
- **Dependencies**: None (but doing this before Task 3.8 makes the split easier)

---

### Task 3.8: Split `launcher.rs` into focused modules
- **Findings**: #27
- **Files to modify**:
  - `crates/photoncast/src/launcher.rs` (7632 lines)
  - New directory: `crates/photoncast/src/launcher/`
- **Description**: `launcher.rs` is 7632 lines — the largest file in the project. Split into focused modules.
- **Implementation Steps**:
  1. Create `crates/photoncast/src/launcher/` directory
  2. Create `mod.rs` with the `LauncherWindow` struct definition and re-exports
  3. Split into sub-modules:
     - `launcher/search.rs` — search input handling, query processing
     - `launcher/render.rs` — GPUI render method and UI layout
     - `launcher/actions.rs` — action menu, action execution
     - `launcher/indexing.rs` — app indexing, icon extraction
     - `launcher/calculator.rs` — calculator integration
     - `launcher/calendar.rs` — calendar/meeting widget
     - `launcher/animation.rs` — window animations
     - `launcher/uninstall.rs` — uninstall preview
  4. Move `impl` blocks to appropriate sub-modules using `impl LauncherWindow { ... }` in each file
  5. Ensure all cross-module field accesses work (may need `pub(crate)` on sub-struct fields)
- **Testing Requirements**:
  - All existing launcher tests pass
  - All launcher functionality works
- **Complexity**: XL
- **Dependencies**: Task 3.7 (sub-structs make splitting easier)

---

### Task 3.9: Split `file_search_view.rs` into sub-modules
- **Findings**: #28
- **Files to modify**:
  - `crates/photoncast/src/file_search_view.rs` (2147 lines)
  - New directory: `crates/photoncast/src/file_search_view/`
- **Description**: Second largest view file at 2147 lines.
- **Implementation Steps**:
  1. Create `crates/photoncast/src/file_search_view/` directory
  2. Split into:
     - `mod.rs` — struct definition, re-exports
     - `filter.rs` — file type filtering logic
     - `render.rs` — GPUI render methods
     - `browsing.rs` — directory navigation
     - `helpers.rs` — utility functions
- **Testing Requirements**:
  - File search view renders and functions correctly
- **Complexity**: Large
- **Dependencies**: Task 3.6 (deduplicate extension lists first)

---

### Task 3.10: Extract `main()` initialization into focused functions
- **Findings**: #25
- **Files to modify**:
  - `crates/photoncast/src/main.rs` (function `main()` starting at line 131)
- **Description**: `main()` is ~400 lines with deeply nested initialization, clipboard setup, hotkey registration, and a ~300-line event loop. Extract into focused functions/structs.
- **Implementation Steps**:
  1. Extract initialization steps into named functions:
     - `fn init_logging()` — tracing setup
     - `fn init_clipboard(config) -> ClipboardState` — clipboard monitor setup
     - `fn init_hotkeys(cx)` — global hotkey registration
     - `fn init_extensions(config) -> ExtensionManager` — extension discovery and loading
  2. Extract the event loop body into a dedicated function or struct method
  3. Keep `main()` as a high-level orchestrator calling these functions
- **Testing Requirements**:
  - App starts and all features work
  - Hotkeys register correctly
- **Complexity**: Large
- **Dependencies**: Task 3.2 (extract window helpers first)

---

### Task 3.11: Verify Phase 3 — Structural Refactoring
- **Description**: Full verification after all refactoring tasks.
- **Implementation Steps**:
  1. Run `cargo clippy --workspace -- -D warnings`
  2. Run `cargo test --workspace`
  3. Run `cargo build --release` — ensure no build regressions
  4. Manual smoke test: launch app, perform search, open file search, use extension
  5. Verify no functionality lost during refactoring
- **Complexity**: Medium
- **Dependencies**: Tasks 3.1–3.10

---

## Phase 4: Code Cleanup & Minor Fixes

Priority: **Low** — These are hygiene improvements that reduce noise and improve maintainability.

---

### Task 4.1: Address blanket clippy `#![allow(...)]` directives
- **Findings**: #31
- **Files to modify**:
  - `crates/photoncast-core/src/lib.rs` (lines 20-65, 44 blanket allows)
- **Description**: 44 blanket clippy allow directives suppress legitimate warnings crate-wide. These should be addressed per-function or removed.
- **Implementation Steps**:
  1. Remove one `#![allow(...)]` at a time from `lib.rs`
  2. Run `cargo clippy` to find the affected locations
  3. For each warning:
     - Fix the code if the fix is trivial (e.g., `uninlined_format_args`, `redundant_closure`)
     - Add `#[allow(...)]` locally with a comment if the lint is intentionally suppressed
  4. Group related allows: keep only those that are genuinely project-wide style choices (e.g., `module_name_repetitions`)
  5. Target: reduce from 44 to ≤10 blanket allows
- **Testing Requirements**:
  - `cargo clippy --workspace` passes with fewer blanket allows
  - No new warnings introduced
- **Complexity**: Large (many individual fixes, but each is small)
- **Dependencies**: All Phase 1-3 tasks (to avoid merge conflicts)

---

### Task 4.2: Add `From` implementations for `TimerError`
- **Findings**: #36
- **Files to modify**:
  - `crates/photoncast-timer/src/error.rs`
- **Description**: Unlike other error types in the project, `TimerError` requires manual string conversion at every call site.
- **Implementation Steps**:
  1. Add `impl From<std::io::Error> for TimerError` and other common conversions
  2. Consider using `thiserror` derive macros if not already used
  3. Update call sites to use `?` operator instead of manual `.map_err(|e| TimerError::...)`
- **Testing Requirements**:
  - All timer-related tests pass
  - Error conversion compiles correctly
- **Complexity**: Small
- **Dependencies**: None

---

### Task 4.3: Document `ClipboardState.monitor` dead_code suppression
- **Findings**: #34
- **Files to modify**:
  - `crates/photoncast/src/main.rs` (lines 127-128)
- **Description**: `monitor` field has `#[allow(dead_code)]` without explanation. It's kept alive to maintain the background monitoring task.
- **Implementation Steps**:
  1. Replace `#[allow(dead_code)]` with a comment:
     ```rust
     /// Kept alive to maintain the background clipboard monitoring task.
     /// Dropping this would stop clipboard change detection.
     #[allow(dead_code)] // Field is intentionally kept for its Drop side-effect
     monitor: Option<Arc<ClipboardMonitor>>,
     ```
- **Testing Requirements**:
  - Compiles without warnings
- **Complexity**: Small
- **Dependencies**: None

---

### Task 4.4: Remove or document dead `resize_window_height` / `get_window_height`
- **Findings**: #39
- **Files to modify**:
  - `crates/photoncast/src/platform.rs` (lines 32 and 59)
- **Description**: Both functions are marked `#[allow(dead_code)]`. Either remove them or document why they're kept.
- **Implementation Steps**:
  1. Check if these functions are used anywhere (grep for callers)
  2. If unused and not planned for future use, remove them
  3. If planned for future use, add a doc comment explaining the intent and keep the allow
- **Testing Requirements**:
  - Compiles without errors
- **Complexity**: Small
- **Dependencies**: None

---

### Task 4.5: Extract magic numbers into named constants
- **Findings**: #38
- **Files to modify**:
  - Various files across the codebase
- **Description**: Hardcoded values like overlay alpha `0.6`, icon sizes, animation durations, etc., are scattered without named constants.
- **Implementation Steps**:
  1. Grep for common magic numbers: `0.6`, `px(16`, `px(24`, `px(32`, `px(48`, common animation durations
  2. For each, define a named constant in the appropriate `constants.rs` or at the top of the file
  3. Replace magic numbers with the constant references
  4. Focus on values used in more than one place first
- **Testing Requirements**:
  - All UI renders correctly
  - No visual regressions
- **Complexity**: Medium
- **Dependencies**: Task 3.5 (deduplicate constants first)

---

### Task 4.6: Document extension sandboxing as architectural concern
- **Findings**: #9
- **Files to modify**:
  - `crates/photoncast-core/src/extensions/manager.rs` (add doc comment)
  - Consider adding `ARCHITECTURE.md` or a section in existing docs
- **Description**: Extensions run in-process with full host privileges. This is a long-term architectural concern that should be documented.
- **Implementation Steps**:
  1. Add a `# Security` section to the module-level doc comment in `manager.rs`
  2. Document:
     - Current limitation: extensions run in-process
     - Mitigations in place: permissions system, code signing (after Task 1.3)
     - Future direction: consider process-based sandboxing
  3. Create a tracking issue (or TODO comment) for future sandboxing work
- **Testing Requirements**:
  - Documentation compiles (`cargo doc`)
- **Complexity**: Small
- **Dependencies**: Task 1.3 (so documentation can reference signing verification)

---

### Task 4.7: Cache or pool FuzzyMatcher in ExtensionManager::search
- **Findings**: #15
- **Files to modify**:
  - `crates/photoncast-core/src/extensions/manager.rs` (function `search()`, around the `let mut matcher = ...` lines)
- **Description**: `search()` creates new `FuzzyMatcher` instances on every keystroke. While the current code already creates them as local variables inside `search()`, the matchers could be cached on the struct.
- **Implementation Steps**:
  1. Add `matcher: FuzzyMatcher` and `command_matcher: FuzzyMatcher` fields to `ExtensionManager`
  2. In `search()`, use `&mut self.matcher` and `&mut self.command_matcher` instead of creating new ones
  3. If `FuzzyMatcher` has internal state that needs resetting between searches, call a `reset()` method or clear it
  4. Alternatively, if `search()` takes `&self`, this would require interior mutability — evaluate if the savings justify the complexity
- **Testing Requirements**:
  - Search results are identical to before
  - No stale state between searches
- **Complexity**: Small
- **Dependencies**: None

---

### Task 4.8: Verify Phase 4 — Code Cleanup
- **Description**: Final verification of all code cleanup tasks.
- **Implementation Steps**:
  1. Run `cargo clippy --workspace -- -D warnings`
  2. Run `cargo test --workspace`
  3. Run `cargo doc --workspace --no-deps` — verify documentation compiles
  4. Verify reduced clippy allow count
  5. Final smoke test of the application
- **Complexity**: Small
- **Dependencies**: Tasks 4.1–4.7

---

## Summary

| Phase | Tasks | Findings Covered | Est. Effort |
|-------|-------|-----------------|-------------|
| Phase 1: Security Hardening | 8 (incl. verify) | #1-#9 | Medium |
| Phase 2: Critical Performance | 13 (incl. verify) | #10-#24 | Large |
| Phase 3: Structural Refactoring | 11 (incl. verify) | #25-#30, #32-#33, #35, #37 | XL |
| Phase 4: Code Cleanup | 8 (incl. verify) | #9, #15, #31, #34, #36, #38-#39 | Medium |
| **Total** | **40 tasks** | **All 39 findings** | |

### Critical Path
1. Phase 1 tasks are independent and can be parallelized
2. Phase 2 tasks are mostly independent (except 2.11 depends on 2.8)
3. Phase 3 has key dependencies: 3.7 → 3.8, 3.6 → 3.9, 3.2 → 3.10
4. Phase 4 should wait for all structural changes to avoid merge conflicts
