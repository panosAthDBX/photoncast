# Search & UX Improvements — Technical Specification

## Status: 📋 Ready for Implementation

**Date**: 2026-02-11  
**Branch**: TBD (off `rust-perf-recos-20260209`)  
**Author**: Spec Writer Droid  

---

## 1. Overview

This specification covers four improvements to PhotonCast's search, indexing, and app management UX. Each improvement is self-contained and ordered by implementation priority:

1. **Smarter App Indexing** — Filter system helper apps; user-configurable search scope
2. **Frecency-Based Result Sorting** — Stronger frecency weight; per-query frecency tracking
3. **Better Fuzzy Matching** — Word-boundary/acronym bonus scoring
4. **Faster App Quitting** — Fire-and-forget terminate; silent dismiss

### Objectives

- Eliminate noise from system helper apps in search results
- Make frequently-used apps consistently outrank better string matches
- Enable acronym-style queries (e.g., "vsc" → Visual Studio Code)
- Remove the 5-second blocking quit timeout for instant UX

### Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (2021 edition) |
| GUI | GPUI + gpui-component |
| Fuzzy Matching | nucleo |
| Storage | rusqlite (SQLite, WAL mode) |
| Async | Tokio |
| Error Handling | thiserror + anyhow |
| macOS APIs | objc2, cocoa, plist |
| Testing | proptest, criterion |

---

## 2. Phase 1: Smarter App Indexing

**Priority**: Highest  
**Estimated scope**: ~150 lines changed across 3 files

### 2.1 Problem

`/System/Library/CoreServices` indexes ~50+ items including system helpers (Archive Utility, Setup Assistant, Bluetooth File Exchange, etc.) that pollute search results. Users cannot customize which directories are scanned.

### 2.2 Design

#### 2.2.1 CoreServices LSUIElement Filtering

**File**: `crates/photoncast-core/src/indexer/scanner.rs`

Currently, `find_app_bundles()` iterates directory entries and applies exclusion patterns via `is_excluded()`. A new filtering step must check the `LSUIElement` key in each CoreServices app's `Info.plist`.

**New function to add in `scanner.rs`**:

```rust
/// Returns true if the app at `path` is a background-only app (LSUIElement=true).
///
/// Only called for apps found in /System/Library/CoreServices.
/// Background apps should be excluded from the search index.
async fn is_background_app(app_path: &Path) -> bool {
    let info_plist_path = app_path.join("Contents/Info.plist");
    let Ok(contents) = tokio::fs::read(&info_plist_path).await else {
        return false; // If we can't read, include the app (fail open)
    };
    let Ok(plist_value) = plist::from_bytes::<plist::Value>(&contents) else {
        return false;
    };
    let Some(dict) = plist_value.as_dictionary() else {
        return false;
    };
    // LSUIElement=true means background-only app (no Dock icon, no menu bar)
    dict.get("LSUIElement")
        .map(|v| {
            // Can be bool true or string "1" or integer 1
            v.as_boolean().unwrap_or(false)
                || v.as_string().is_some_and(|s| s == "1")
                || v.as_signed_integer().is_some_and(|n| n == 1)
                || v.as_unsigned_integer().is_some_and(|n| n == 1)
        })
        .unwrap_or(false)
}
```

**Integration point in `find_app_bundles()`** — after the resolved path passes `is_app_bundle()` and `is_excluded()` checks, but before deduplication:

```rust
// NEW: Filter background apps from CoreServices
let is_core_services = path.starts_with("/System/Library/CoreServices");
if is_core_services && is_background_app(&resolved).await {
    debug!("Excluding background CoreServices app: {}", resolved.display());
    continue;
}
```

**Note on `find_app_bundles` signature**: This method currently returns `Result<Vec<PathBuf>>` and is called in a `buffer_unordered` stream. The `is_background_app` call is already within an async context, so no signature change is needed.

**Crate dependency**: The `plist` crate is already used in `crates/photoncast-core/src/indexer/metadata.rs` — no new dependency needed.

#### 2.2.2 User-Configurable Search Scope

**File**: `crates/photoncast-core/src/app/config.rs`

Add a new `app_search_scope` field to the existing `SearchConfig` struct:

```rust
/// Search configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    // ... existing fields ...

    /// Custom directories to scan for applications.
    /// If empty, defaults are used.
    /// Changes trigger a re-index.
    #[serde(default)]
    pub app_search_scope: Vec<PathBuf>,
}
```

**Default value**: When `app_search_scope` is empty, use the existing `SCAN_PATHS` constant from `scanner.rs`.

**File**: `crates/photoncast-core/src/indexer/scanner.rs`

Modify `AppScanner::new()` to accept an optional custom scope:

```rust
/// Creates a new scanner from configuration.
/// If `custom_paths` is non-empty, uses those instead of SCAN_PATHS.
pub fn from_config(custom_paths: &[PathBuf]) -> Self {
    let scan_paths = if custom_paths.is_empty() {
        SCAN_PATHS
            .iter()
            .map(|p| {
                if p.starts_with('~') {
                    dirs::home_dir()
                        .map_or_else(|| PathBuf::from(p), |h| h.join(&p[2..]))
                } else {
                    PathBuf::from(p)
                }
            })
            .collect()
    } else {
        custom_paths
            .iter()
            .map(|p| {
                let s = p.to_string_lossy();
                if s.starts_with('~') {
                    dirs::home_dir()
                        .map_or_else(|| p.clone(), |h| h.join(&s[2..]))
                } else {
                    p.clone()
                }
            })
            .collect()
    };

    Self {
        scan_paths,
        excluded_patterns: EXCLUDED_PATTERNS.iter().map(|s| (*s).to_string()).collect(),
        timeout: SCAN_TIMEOUT,
    }
}
```

**Re-index trigger**: When the config changes, the code that constructs `AppScanner` already rebuilds from the new config. The existing file-watcher on the config file (`crates/photoncast-core/src/indexer/watcher.rs`) or the preference save handler should call `scan_all()` again with the new scanner.

### 2.3 Testing Strategy

| Test | Type | Description |
|------|------|-------------|
| `test_is_background_app_with_ls_ui_element_true` | Unit | Verify apps with `LSUIElement=true` are detected |
| `test_is_background_app_with_ls_ui_element_false` | Unit | Verify apps with `LSUIElement=false` are included |
| `test_is_background_app_with_missing_key` | Unit | Verify apps without LSUIElement are included (fail open) |
| `test_is_background_app_integer_value` | Unit | Verify `LSUIElement=1` (integer) is detected |
| `test_scanner_from_config_empty` | Unit | Verify empty config uses SCAN_PATHS |
| `test_scanner_from_config_custom` | Unit | Verify custom paths are used |
| `test_scanner_from_config_tilde_expansion` | Unit | Verify `~` is expanded in custom paths |
| Property: `is_background_app` deterministic | proptest | Same plist always returns same result |

### 2.4 Success Criteria

- [ ] System helper apps (Archive Utility, Setup Assistant, Bluetooth File Exchange) do not appear in search
- [ ] Finder.app, Activity Monitor, and other user-facing CoreServices apps still appear
- [ ] Users can add/remove directories from search scope in `config.toml`
- [ ] Changing search scope triggers re-indexing

---

## 3. Phase 2: Frecency-Based Result Sorting

**Priority**: High  
**Estimated scope**: ~350 lines changed across 5 files + 1 new table

### 3.1 Problem

Typing "sh" shows "Shortcuts" (higher match score) instead of "Shortwave" (frequently used). The current `FRECENCY_MULTIPLIER` of `10.0` is insufficient to override match quality differences. There is no per-query frecency tracking.

### 3.2 Design

#### 3.2.1 Increase FRECENCY_MULTIPLIER

**File**: `crates/photoncast-core/src/search/ranking.rs`

```rust
impl ResultRanker {
    /// Frecency multiplier in the combined score formula.
    /// Bumped from 10.0 to 35.0 to ensure heavily-used apps dominate match quality.
    pub const FRECENCY_MULTIPLIER: f64 = 35.0;
    // ...
}
```

**Rationale for 35.0**: A typical nucleo match score difference between a prefix match and a fuzzy match is ~20-80 points. A frecency of 5.0 (5 launches, recent) × 35.0 = 175 points — enough to overcome the match quality gap. This is in the 25-50x range specified in decisions.

**File**: `crates/photoncast-core/src/search/providers/optimized_apps.rs`

Three locations currently hardcode `10.0`:

```rust
// Line 166-167: Sort comparison
let combined_a = frecency_a.mul_add(10.0, f64::from(a.1));  // → MULTIPLIER
let combined_b = frecency_b.mul_add(10.0, f64::from(b.1));  // → MULTIPLIER

// Line 190: Score calculation  
score: entry.frecency.mul_add(10.0, f64::from(score)),       // → MULTIPLIER
```

**Change**: Extract the multiplier to a constant or reference `ResultRanker::FRECENCY_MULTIPLIER`:

```rust
use crate::search::ranking::ResultRanker;

// In sort comparison:
let combined_a = frecency_a.mul_add(ResultRanker::FRECENCY_MULTIPLIER, f64::from(a.1));
let combined_b = frecency_b.mul_add(ResultRanker::FRECENCY_MULTIPLIER, f64::from(b.1));

// In result construction:
score: entry.frecency.mul_add(ResultRanker::FRECENCY_MULTIPLIER, f64::from(score)),
```

#### 3.2.2 Per-Query Frecency Tracking

**Database schema migration (v2)**:

**File**: `crates/photoncast-core/src/storage/database.rs`

Add to `run_migrations()`:

```rust
fn run_migrations(&self) -> Result<()> {
    let current_version = self.get_schema_version()?;

    if current_version < 1 {
        self.migrate_v1()?;
        self.record_version(1)?;
    }
    if current_version < 2 {
        self.migrate_v2()?;
        self.record_version(2)?;
    }

    Ok(())
}
```

Update the `CURRENT_SCHEMA_VERSION`:

```rust
const CURRENT_SCHEMA_VERSION: i32 = 2;
```

New migration function:

```rust
/// Migration v2: Per-query frecency tracking.
fn migrate_v2(&self) -> Result<()> {
    let conn = self.conn.lock();

    conn.execute_batch(
        r"
        -- Per-query frecency: tracks which items are selected for specific query prefixes
        CREATE TABLE IF NOT EXISTS query_frecency (
            query_prefix TEXT NOT NULL,
            item_id TEXT NOT NULL,
            frequency INTEGER NOT NULL DEFAULT 1,
            last_used_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (query_prefix, item_id)
        );

        CREATE INDEX IF NOT EXISTS idx_query_frecency_item
        ON query_frecency(item_id);

        CREATE INDEX IF NOT EXISTS idx_query_frecency_last_used
        ON query_frecency(last_used_at);
        ",
    )
    .context("failed to run migration v2")?;

    Ok(())
}
```

**Database operations to add in `database.rs`**:

```rust
/// Records a query→item selection for per-query frecency.
///
/// Stores the association between a query prefix and the selected item.
/// Only tracks prefixes of length 1-4 characters.
pub fn record_query_selection(&self, query_prefix: &str, item_id: &str) -> Result<()> {
    if query_prefix.is_empty() || query_prefix.len() > 4 {
        return Ok(()); // Only track prefixes 1-4 chars
    }

    let conn = self.conn.lock();
    let now = Utc::now().timestamp();

    conn.execute(
        r"
        INSERT INTO query_frecency (query_prefix, item_id, frequency, last_used_at, created_at)
        VALUES (?1, ?2, 1, ?3, ?3)
        ON CONFLICT(query_prefix, item_id) DO UPDATE SET
            frequency = frequency + 1,
            last_used_at = ?3
        ",
        rusqlite::params![query_prefix, item_id, now],
    )
    .context("failed to record query selection")?;

    Ok(())
}

/// Gets the per-query frecency score for an item given a query prefix.
///
/// Returns (frequency, last_used_at) if found.
pub fn get_query_frecency(&self, query_prefix: &str, item_id: &str) -> Result<Option<(u32, i64)>> {
    let conn = self.conn.lock();

    let result = conn.query_row(
        "SELECT frequency, last_used_at FROM query_frecency WHERE query_prefix = ?1 AND item_id = ?2",
        rusqlite::params![query_prefix, item_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok(data) => Ok(Some(data)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to get query frecency"),
    }
}

/// Prunes old query frecency entries (older than 30 days).
pub fn prune_query_frecency(&self, max_age_days: i64) -> Result<usize> {
    let conn = self.conn.lock();
    let cutoff = Utc::now().timestamp() - (max_age_days * 86400);

    let deleted = conn
        .execute(
            "DELETE FROM query_frecency WHERE last_used_at < ?1",
            rusqlite::params![cutoff],
        )
        .context("failed to prune query frecency")?;

    Ok(deleted)
}
```

#### 3.2.3 UsageTracker Integration

**File**: `crates/photoncast-core/src/storage/usage.rs`

Add new methods to `UsageTracker`:

```rust
impl UsageTracker {
    /// Records a query prefix → item selection for per-query frecency.
    ///
    /// Extracts prefixes of length 1 through min(4, query.len()) and records
    /// each one as a separate association.
    pub fn record_query_selection(&self, query: &str, item_id: &str) -> Result<()> {
        let query_lower = query.to_lowercase();
        let max_prefix_len = query_lower.len().min(4);

        for len in 1..=max_prefix_len {
            // Ensure we split on char boundaries
            if let Some(prefix) = query_lower.get(..len) {
                self.db.record_query_selection(prefix, item_id)?;
            }
        }
        Ok(())
    }

    /// Records a query prefix → item selection asynchronously.
    pub async fn record_query_selection_async(&self, query: String, item_id: String) -> Result<()> {
        let db = self.db.clone();
        task::spawn_blocking(move || {
            let tracker = UsageTracker::new(db);
            tracker.record_query_selection(&query, &item_id)
        })
        .await?
    }

    /// Gets the per-query frecency score for an item.
    ///
    /// Looks up the query prefix (truncated to 4 chars max) and returns
    /// a FrecencyScore using the same 72-hour half-life.
    pub fn get_query_frecency(&self, query: &str, item_id: &str) -> Result<FrecencyScore> {
        let query_lower = query.to_lowercase();
        let prefix_len = query_lower.len().min(4);
        let Some(prefix) = query_lower.get(..prefix_len) else {
            return Ok(FrecencyScore::zero());
        };

        match self.db.get_query_frecency(prefix, item_id)? {
            Some((frequency, last_used_ts)) => {
                let last_used = timestamp_to_system_time(last_used_ts);
                Ok(FrecencyScore::calculate(frequency, last_used))
            }
            None => Ok(FrecencyScore::zero()),
        }
    }
}
```

#### 3.2.4 Ranking Integration

**File**: `crates/photoncast-core/src/search/ranking.rs`

Update the combined score formula to include per-query frecency:

```rust
impl ResultRanker {
    /// Calculates the combined score with per-query frecency support.
    ///
    /// Formula: `final_score = (match_score + (global_frecency + query_frecency) * MULTIPLIER) * boosts`
    #[must_use]
    pub fn calculate_combined_score_with_query(
        &self,
        match_score: f64,
        global_frecency: &FrecencyScore,
        query_frecency: &FrecencyScore,
        query: &str,
        title: &str,
        path: Option<&Path>,
    ) -> f64 {
        let combined_frecency = global_frecency.score() + query_frecency.score();
        let base_score = combined_frecency.mul_add(Self::FRECENCY_MULTIPLIER, match_score);
        self.apply_boosts(base_score, query, title, path)
    }
}
```

#### 3.2.5 Recording Selections

When the user activates a search result, record both the app launch (global frecency) and the query→item association (per-query frecency).

**File**: `crates/photoncast/src/launcher/actions.rs` (or the `activate` handler in the launcher)

In the `activate` handler (the code that runs when the user presses Enter on a search result), after the existing `record_app_launch()` call, add:

```rust
// Record per-query frecency (query → selected item association)
if !self.search.query.is_empty() {
    if let Some(result) = self.search.results.get(self.search.selected_index) {
        let query = self.search.query.clone();
        let item_id = result.bundle_id.clone().unwrap_or_default();
        if !item_id.is_empty() {
            // Fire and forget — don't block on this
            let usage_tracker = self.usage_tracker.clone();
            cx.spawn(|_, _| async move {
                if let Err(e) = usage_tracker.record_query_selection_async(query, item_id).await {
                    tracing::warn!("Failed to record query selection: {}", e);
                }
            }).detach();
        }
    }
}
```

#### 3.2.6 OptimizedAppProvider Query-Aware Search

The `OptimizedAppProvider::search()` method needs access to the current query for per-query frecency lookup. Two approaches:

**Approach (recommended)**: Add a method that accepts a `UsageTracker` reference:

```rust
impl OptimizedAppProvider {
    /// Search with per-query frecency boost.
    pub fn search_with_frecency(
        &self,
        query: &str,
        max_results: usize,
        usage_tracker: Option<&UsageTracker>,
    ) -> Vec<SearchResult> {
        // ... same as current search() but in the sort step:
        // Look up query frecency for each result
        // Add it to the combined score
    }
}
```

This avoids changing the `SearchProvider` trait while enabling per-query frecency in the app provider.

### 3.3 Testing Strategy

| Test | Type | Description |
|------|------|-------------|
| `test_frecency_multiplier_overcomes_match_quality` | Unit | App with 5 launches beats higher match score |
| `test_record_query_selection` | Unit | DB stores query→item mapping |
| `test_query_frecency_lookup` | Unit | Correct frecency returned for prefix |
| `test_query_prefix_truncation` | Unit | Only prefixes 1-4 chars stored |
| `test_query_frecency_decay` | Unit | Older selections have lower score |
| `test_migration_v2_additive` | Unit | v1 data preserved after v2 migration |
| `test_prune_old_entries` | Unit | Entries older than 30 days removed |
| `test_combined_score_formula` | Unit | global + query frecency combined correctly |
| Property: frecency always non-negative | proptest | `FrecencyScore::score() >= 0` |
| Benchmark: ranking with frecency | criterion | Ensure < 1ms for 200 results |

### 3.4 Success Criteria

- [ ] "sh" → Shortwave at top when Shortwave has 5+ recent launches
- [ ] Per-query frecency: selecting "Shortwave" for "sh" boosts it for future "sh" queries
- [ ] Migration v2 is additive (no changes to existing tables)
- [ ] No regression in search latency (benchmark p99 < 2ms)

---

## 4. Phase 3: Better Fuzzy Matching

**Priority**: Medium  
**Estimated scope**: ~120 lines changed in 1 file + tests

### 4.1 Problem

nucleo handles word boundaries internally but doesn't give explicit bonuses for matching initial letters of words. Acronym-style queries ("ss" → System Settings, "vsc" → Visual Studio Code) don't get the expected boost.

### 4.2 Design

#### 4.2.1 Word-Boundary/Acronym Bonus

**File**: `crates/photoncast-core/src/search/fuzzy.rs`

Add a new function and integrate it into `FuzzyMatcher::score()`:

```rust
/// Configuration for word boundary bonus.
const WORD_BOUNDARY_BONUS: u32 = 20;

/// Detects word boundary positions in a string.
///
/// Word boundaries are:
/// - Start of string (index 0)
/// - After space, hyphen, underscore, dot, slash
/// - CamelCase transitions (lowercase → uppercase)
fn find_word_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = Vec::new();
    let chars: Vec<char> = text.chars().collect();

    if chars.is_empty() {
        return boundaries;
    }

    // First character is always a boundary
    boundaries.push(0);

    for i in 1..chars.len() {
        let prev = chars[i - 1];
        let curr = chars[i];

        // After separator characters
        if prev == ' ' || prev == '-' || prev == '_' || prev == '.' || prev == '/' {
            boundaries.push(i);
        }
        // CamelCase transition: lowercase followed by uppercase
        else if prev.is_lowercase() && curr.is_uppercase() {
            boundaries.push(i);
        }
    }

    boundaries
}

/// Calculates the word-boundary bonus for a match.
///
/// Counts how many of the matched character positions fall on word boundaries.
/// The bonus is proportional to the ratio of boundary matches to total query length.
///
/// For example:
/// - "ss" → "System Settings": matches at positions [0, 7], boundaries at [0, 7]
///   → 2/2 boundary matches = full bonus (WORD_BOUNDARY_BONUS * 2)
/// - "vsc" → "Visual Studio Code": matches at [0, 7, 14], boundaries at [0, 7, 14]
///   → 3/3 boundary matches = full bonus (WORD_BOUNDARY_BONUS * 3)
fn calculate_word_boundary_bonus(target: &str, match_indices: &[usize]) -> u32 {
    if match_indices.is_empty() {
        return 0;
    }

    let target_lower = target.to_lowercase();
    let boundaries = find_word_boundaries(&target_lower);
    let boundary_set: std::collections::HashSet<usize> = boundaries.into_iter().collect();

    let boundary_matches = match_indices
        .iter()
        .filter(|idx| boundary_set.contains(idx))
        .count();

    // Bonus per boundary-matched character
    (boundary_matches as u32) * WORD_BOUNDARY_BONUS
}
```

**Integration into `FuzzyMatcher::score()`**:

After the spread factor check and before returning, add the word-boundary bonus:

```rust
// Apply word boundary/acronym bonus
let boundary_bonus = calculate_word_boundary_bonus(target, &match_indices);
let final_score_with_bonus = final_score + boundary_bonus;

Some((u32::from(final_score_with_bonus), match_indices))
```

**Note**: The bonus is additive on top of nucleo's score. With `WORD_BOUNDARY_BONUS = 20` per boundary match, a 2-char acronym like "ss" → System Settings gets +40, a 3-char like "vsc" → Visual Studio Code gets +60. This is significant enough to promote acronym matches but does not override strong frecency (which contributes hundreds of points at the new 35.0 multiplier).

#### 4.2.2 Configuration

Add `word_boundary_bonus` to `MatcherConfig`:

```rust
pub struct MatcherConfig {
    // ... existing fields ...
    /// Bonus score per character that matches a word boundary.
    /// Set to 0 to disable word-boundary scoring.
    pub word_boundary_bonus: u32,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            smart_case: true,
            normalize_unicode: true,
            prefer_prefix: true,
            max_spread_factor: 1.5,
            word_boundary_bonus: 20,
        }
    }
}
```

### 4.3 Testing Strategy

| Test | Type | Description |
|------|------|-------------|
| `test_word_boundaries_simple` | Unit | "System Settings" → boundaries at [0, 7] |
| `test_word_boundaries_camelcase` | Unit | "macOS" → boundaries at [0, 3] |
| `test_word_boundaries_hyphen` | Unit | "Wi-Fi" → boundaries at [0, 3] |
| `test_acronym_bonus_ss` | Unit | "ss" matching "System Settings" gets bonus |
| `test_acronym_bonus_vsc` | Unit | "vsc" matching "Visual Studio Code" gets bonus |
| `test_acronym_bonus_gc` | Unit | "gc" matching "Google Chrome" gets bonus |
| `test_no_bonus_non_boundary` | Unit | Random mid-word matches get no bonus |
| `test_boundary_bonus_disabled` | Unit | Setting bonus to 0 disables it |
| Property: bonus always non-negative | proptest | `calculate_word_boundary_bonus() >= 0` |
| Benchmark: fuzzy score with bonus | criterion | Ensure < 10µs per match |

### 4.4 Success Criteria

- [ ] "ss" matches System Settings with higher score than without boundary bonus
- [ ] "vsc" matches Visual Studio Code effectively
- [ ] "gc" matches Google Chrome
- [ ] No regression in existing fuzzy match quality (existing tests pass)
- [ ] Spread factor 1.5x unchanged, prefix bonus unchanged

---

## 5. Phase 4: Faster App Quitting

**Priority**: Lower  
**Estimated scope**: ~40 lines changed across 2 files

### 5.1 Problem

`quit_app_by_bundle_id()` sends `terminate()` and then polls every 100ms for up to 5 seconds (`QUIT_TIMEOUT_SECS = 5`), blocking the thread. `quit_app_with_timeout()` uses a `thread::sleep` loop. The launcher UI freezes during this time.

### 5.2 Design

#### 5.2.1 Fire-and-Forget Quit

**File**: `crates/photoncast-apps/src/process.rs`

Replace `quit_app_by_bundle_id()` to send terminate and return immediately:

```rust
/// Quits an application by its bundle identifier (fire-and-forget).
///
/// Finds the running application with the given bundle ID and sends a terminate
/// signal. Returns immediately without waiting for the app to actually quit.
///
/// # Arguments
///
/// * `bundle_id` - The bundle identifier of the application (e.g., "com.apple.Safari").
///
/// # Returns
///
/// * `Ok(true)` - The terminate signal was sent successfully.
/// * `Err(AppNotRunning)` - No running app with that bundle ID.
#[cfg(target_os = "macos")]
pub fn quit_app_by_bundle_id(bundle_id: &str) -> ActionResult<bool> {
    tracing::info!("Sending fire-and-forget quit to bundle ID: {}", bundle_id);

    let workspace = NSWorkspace::sharedWorkspace();
    let apps = workspace.runningApplications();

    let bundle_id_lower = bundle_id.to_lowercase();
    let count = apps.count();

    for i in 0..count {
        let app = apps.objectAtIndex(i);

        if let Some(app_bundle_id) = app.bundleIdentifier() {
            let app_bundle_id_str = app_bundle_id.to_string();
            if app_bundle_id_str.to_lowercase() == bundle_id_lower {
                let pid = app.processIdentifier();
                tracing::debug!("Found running app '{}' with PID {}", bundle_id, pid);

                // Send terminate and return immediately — no polling loop
                let success = app.terminate();
                if success {
                    tracing::info!("Terminate signal sent to '{}' (PID {})", bundle_id, pid);
                    return Ok(true);
                } else {
                    return Err(ActionError::OperationFailed {
                        operation: "quit".to_string(),
                        reason: format!(
                            "Failed to send terminate to PID {} - app may not support graceful quit",
                            pid
                        ),
                    });
                }
            }
        }
    }

    Err(ActionError::AppNotRunning {
        bundle_id: bundle_id.to_string(),
    })
}
```

**Key changes**:
- Removes the call to `quit_app_with_timeout()` (which had the polling loop)
- Sends `app.terminate()` and returns `Ok(true)` immediately
- `quit_app_with_timeout()` can remain in the codebase for force-quit flows or can be deprecated

#### 5.2.2 Action Handler — Silent Dismiss

**File**: `crates/photoncast/src/launcher/actions.rs`

The quit action handler already calls `self.hide(cx)` after quitting. The only change is that `quit_app_by_bundle_id` now returns immediately, so the hide happens faster:

```rust
// In quit_app() handler — no change needed, already correct flow:
pub(super) fn quit_app(&mut self, _: &QuitApp, cx: &mut ViewContext<Self>) {
    if let Some(result) = self.search.results.get(self.search.selected_index) {
        if result.result_type == ResultType::Application {
            if let Some(bundle_id) = &result.bundle_id {
                if photoncast_apps::is_app_running(bundle_id) {
                    match photoncast_apps::quit_app_by_bundle_id(bundle_id) {
                        Ok(_) => tracing::info!("Quit app: {}", bundle_id),
                        Err(e) => tracing::error!("Failed to quit app: {}", e),
                    }
                }
            }
        }
    }
    cx.notify();  // This is already fast now — no 5s wait
}
```

Similarly, the actions menu handler (index 5 when running) already calls `self.hide(cx)` after quit — it will now execute immediately.

**No toast, no notification**: The existing code already doesn't show a toast for quit. No changes needed.

### 5.3 Testing Strategy

| Test | Type | Description |
|------|------|-------------|
| `test_quit_by_bundle_id_not_running` | Unit | Returns AppNotRunning error (existing, no change) |
| `test_quit_returns_immediately` | Unit | Verify no sleep/poll in the function |
| `test_force_quit_unchanged` | Unit | Force quit path still works |

### 5.4 Success Criteria

- [ ] Quit action returns in < 10ms (no polling loop)
- [ ] Launcher dismisses immediately after quit action
- [ ] No UI freeze or jank
- [ ] `terminate()` signal is still sent (apps still quit)
- [ ] `force_quit_app()` and `force_quit_app_action()` paths unchanged

---

## 6. Database Schema

### 6.1 Current Schema (v1)

```sql
-- Schema version tracking
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);

-- App usage (global frecency)
CREATE TABLE app_usage (
    bundle_id TEXT PRIMARY KEY,
    launch_count INTEGER NOT NULL DEFAULT 0,
    last_launched_at INTEGER,
    created_at INTEGER NOT NULL
);

-- Command usage
CREATE TABLE command_usage (
    command_id TEXT PRIMARY KEY,
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_at INTEGER,
    created_at INTEGER NOT NULL
);

-- File usage
CREATE TABLE file_usage (
    file_path TEXT PRIMARY KEY,
    open_count INTEGER NOT NULL DEFAULT 0,
    last_opened_at INTEGER,
    created_at INTEGER NOT NULL
);

-- App index cache
CREATE TABLE app_cache (
    bundle_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    icon_path TEXT,
    keywords TEXT,
    category TEXT,
    last_modified INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL
);
```

### 6.2 New Table (v2 Migration)

```sql
-- Per-query frecency: tracks query prefix → item selection associations
CREATE TABLE query_frecency (
    query_prefix TEXT NOT NULL,       -- Lowercase query prefix (1-4 chars)
    item_id TEXT NOT NULL,            -- Bundle ID or item identifier
    frequency INTEGER NOT NULL DEFAULT 1,
    last_used_at INTEGER NOT NULL,    -- Unix timestamp
    created_at INTEGER NOT NULL,      -- Unix timestamp
    PRIMARY KEY (query_prefix, item_id)
);

CREATE INDEX idx_query_frecency_item ON query_frecency(item_id);
CREATE INDEX idx_query_frecency_last_used ON query_frecency(last_used_at);
```

### 6.3 Storage Estimation

- **Row size**: ~80 bytes (prefix 4B + item_id ~40B + integers 24B + overhead)
- **Expected rows**: ~500 active entries (50 apps × ~10 query prefixes each)
- **Storage**: ~40 KB active data
- **Pruning**: Entries older than 30 days auto-pruned

---

## 7. Configuration Changes

### 7.1 New Config Fields

**File**: `crates/photoncast-core/src/app/config.rs`

```toml
# config.toml

[search]
# Existing fields...
include_system_apps = true
file_result_limit = 5

# NEW: Custom app search directories
# If non-empty, replaces the default SCAN_PATHS.
# Changes trigger a re-index.
app_search_scope = []
```

The `app_search_scope` field is empty by default (uses built-in `SCAN_PATHS`). When populated, it replaces the default paths entirely.

### 7.2 Default SCAN_PATHS (reference)

These are the built-in defaults when `app_search_scope` is empty:

```
/Applications
/Applications/Utilities
/System/Applications
/System/Applications/Utilities
/System/Library/CoreServices  (with LSUIElement filtering)
~/Applications
```

---

## 8. File Change Summary

| File | Phase | Changes |
|------|-------|---------|
| `crates/photoncast-core/src/indexer/scanner.rs` | 1 | Add `is_background_app()`, `from_config()`, integrate filtering |
| `crates/photoncast-core/src/app/config.rs` | 1 | Add `app_search_scope` to `SearchConfig` |
| `crates/photoncast-core/src/search/ranking.rs` | 2 | Bump `FRECENCY_MULTIPLIER` to 35.0, add `calculate_combined_score_with_query()` |
| `crates/photoncast-core/src/search/providers/optimized_apps.rs` | 2 | Use `ResultRanker::FRECENCY_MULTIPLIER` constant, add `search_with_frecency()` |
| `crates/photoncast-core/src/storage/database.rs` | 2 | Add migration v2, `record_query_selection()`, `get_query_frecency()`, `prune_query_frecency()` |
| `crates/photoncast-core/src/storage/usage.rs` | 2 | Add `record_query_selection()`, `get_query_frecency()` with prefix extraction |
| `crates/photoncast/src/launcher/actions.rs` | 2 | Record per-query frecency on result activation |
| `crates/photoncast-core/src/search/fuzzy.rs` | 3 | Add `find_word_boundaries()`, `calculate_word_boundary_bonus()`, integrate into `score()` |
| `crates/photoncast-apps/src/process.rs` | 4 | Simplify `quit_app_by_bundle_id()` to fire-and-forget |

---

## 9. Testing Strategy Overview

### 9.1 Unit Tests

- All new functions get unit tests in their respective `#[cfg(test)]` modules
- Existing tests must continue to pass (especially ranking and fuzzy matching tests)
- Update existing ranking tests that rely on `FRECENCY_MULTIPLIER = 10.0`

### 9.2 Property Tests (proptest)

- Word boundary detection is deterministic
- Frecency scores are always non-negative
- Ranking order is deterministic for identical inputs
- Migration is idempotent

### 9.3 Benchmarks (criterion)

- `fuzzy_match_with_boundary_bonus` — Ensure < 10µs per match
- `ranking_with_query_frecency` — Ensure < 2ms for 200 results
- `query_frecency_db_lookup` — Ensure < 100µs per lookup

### 9.4 Integration Tests

- Full scan with CoreServices filtering (verify known background apps excluded)
- Record query selections → verify ranking changes
- Config change → re-index with new paths

---

## 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| High frecency multiplier over-boosts rarely-used apps | Poor results for apps with stale history | 72-hour half-life ensures stale entries decay; per-query frecency prevents global pollution |
| Per-query frecency table grows unbounded | Slow DB queries | Prune entries >30 days old; only store prefixes 1-4 chars; estimated ~500 active rows |
| LSUIElement check adds scan latency | Slower app indexing | Only applies to `/System/Library/CoreServices` (~50 apps); plist parsing is fast (<1ms each) |
| Word boundary bonus interferes with existing scoring | Unexpected ranking changes | Bonus is additive and moderate (20 pts/char); dominated by frecency; configurable |
| Fire-and-forget quit misses "Save changes?" dialogs | User confusion | Expected macOS behavior; user sees dialog when switching to the app |
| Schema migration v2 fails | App crash on startup | Wrapped in error handling; additive only (new table, no changes to existing) |
| Existing tests break after multiplier change | CI failures | Update expected values in affected tests |

---

## 11. Implementation Order

```
Phase 1: Smarter App Indexing
  ├── 1a. Add is_background_app() to scanner.rs
  ├── 1b. Integrate LSUIElement filtering into find_app_bundles()
  ├── 1c. Add from_config() to AppScanner
  ├── 1d. Add app_search_scope to SearchConfig
  ├── 1e. Wire config → scanner
  └── 1f. Tests

Phase 2: Frecency-Based Sorting
  ├── 2a. Bump FRECENCY_MULTIPLIER to 35.0
  ├── 2b. Update optimized_apps.rs to use constant
  ├── 2c. Add migration v2 (query_frecency table)
  ├── 2d. Add DB operations (record/get/prune)
  ├── 2e. Add UsageTracker methods for per-query frecency
  ├── 2f. Add calculate_combined_score_with_query() to ranking.rs
  ├── 2g. Record selections in launcher actions
  ├── 2h. Integrate query frecency into search flow
  ├── 2i. Update existing tests for new multiplier
  └── 2j. New tests + benchmarks

Phase 3: Better Fuzzy Matching
  ├── 3a. Add find_word_boundaries() to fuzzy.rs
  ├── 3b. Add calculate_word_boundary_bonus()
  ├── 3c. Add word_boundary_bonus to MatcherConfig
  ├── 3d. Integrate bonus into FuzzyMatcher::score()
  └── 3e. Tests + benchmarks

Phase 4: Faster App Quitting
  ├── 4a. Simplify quit_app_by_bundle_id() to fire-and-forget
  ├── 4b. Verify action handlers work with instant return
  └── 4c. Tests
```

---

## 12. Glossary

| Term | Definition |
|------|-----------|
| **Frecency** | Combination of frequency and recency: `frequency × recency_decay` |
| **Per-query frecency** | Frecency tracked per search query prefix (e.g., "sh" → Shortwave) |
| **LSUIElement** | macOS Info.plist key; when `true`, app runs as background-only (no Dock, no menu bar) |
| **Word boundary** | Start of a word: first char, after separator, or camelCase transition |
| **Acronym match** | Query characters matching only word boundary positions (e.g., "vsc" → **V**isual **S**tudio **C**ode) |
| **Fire-and-forget** | Send signal and return immediately without waiting for completion |
| **nucleo** | High-performance fuzzy matching library used by Helix editor |
| **Half-life** | Time for recency score to decay by 50% (72 hours in PhotonCast) |
