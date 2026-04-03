# Requirements: Search & UX Improvements

## Status: ✅ Finalized — Ready for Spec Writing

## Overview

Four improvements to PhotonCast's search, indexing, and app management UX, ordered by implementation priority.

---

## Improvement 1: Smarter App Indexing (Priority: Highest)

### Problem
`/System/Library/CoreServices` indexes ~50+ items including system helper apps (Archive Utility, Setup Assistant, Bluetooth File Exchange, etc.) that pollute search results.

### Requirements

#### 1.1 CoreServices LSUIElement Filtering
- **Keep** `/System/Library/CoreServices` in SCAN_PATHS
- **Add filtering**: Only index `.app` bundles from CoreServices where `LSUIElement` is `false` (or absent) in their `Info.plist`
- Apps with `LSUIElement=true` are background/helper apps and should be excluded
- This preserves Finder.app and other user-facing CoreServices apps

#### 1.2 User-Configurable Search Scope
- Add a "Search Scope" preference setting where users can:
  - See the current list of scanned directories
  - Add custom directories to the scan list
  - Remove directories from the scan list (including defaults)
- Default directories remain: `/Applications`, `/Applications/Utilities`, `/System/Applications`, `/System/Applications/Utilities`, `/System/Library/CoreServices` (filtered), `~/Applications`
- Changes to search scope should trigger a re-index

### Technical Approach
- **Scanner changes** (`photoncast-core/src/indexer/scanner.rs`):
  - Add `Info.plist` parsing for CoreServices apps to check `LSUIElement` key
  - Filter out apps where `LSUIElement=true` before adding to index
- **Preferences changes**:
  - Add `search_scope: Vec<PathBuf>` to app preferences/config
  - Wire up to scanner to use configurable paths instead of hardcoded SCAN_PATHS
  - Provide default values matching current SCAN_PATHS

### Success Criteria
- System helper apps (Archive Utility, Setup Assistant, etc.) no longer appear in search
- Finder.app and other user-facing CoreServices apps still appear
- Users can add/remove directories from search scope in preferences
- Changing search scope triggers re-indexing

---

## Improvement 2: Frecency-Based Result Sorting (Priority: High)

### Problem
Typing "sh" shows "Shortcuts" at the top (higher match score) instead of "Shortwave" (frequently used). The current `FRECENCY_MULTIPLIER` of 10.0 is insufficient to overcome match quality differences.

### Requirements

#### 2.1 Increased Frecency Weight
- Increase `FRECENCY_MULTIPLIER` from 10.0 to 25-50x range
- Ensure heavily-used apps consistently outrank slightly better string matches
- Calibrate so that apps with significant usage history (5+ launches recently) always dominate pure match quality
- Also update the `OptimizedAppProvider` sort formula which currently uses `frecency * 10.0 + match_score`

#### 2.2 Per-Query Frecency Tracking
- Track frecency **per search query prefix**, not just globally
- When user types "sh" and selects "Shortwave", record that "sh" → "Shortwave" association
- Next time user types "sh", Shortwave gets a per-query frecency boost in addition to global frecency
- Store per-query frecency in the existing SQLite database (requires schema migration)
- Track for query prefixes of length 1-4 characters (covers most disambiguation queries)

### Technical Approach
- **Ranking changes** (`photoncast-core/src/search/ranking.rs`):
  - Increase `FRECENCY_MULTIPLIER` constant to 25-50 range
  - Add per-query frecency lookup in the ranking pipeline
  - Combined formula: `(match_score + (global_frecency + query_frecency) * MULTIPLIER) * boosts`
- **Storage changes** (`photoncast-core/src/storage/usage.rs`):
  - Add `query_frecency` table: `(query_prefix TEXT, item_id TEXT, frequency INTEGER, last_used TIMESTAMP)`
  - Record query prefix → selected item on each app launch
  - Apply same 72-hour half-life recency decay to per-query frecency
- **OptimizedAppProvider** changes:
  - Update sort formula to use new multiplier
  - Pass current query prefix to enable per-query frecency lookup
- Keep 72-hour half-life for recency decay (no change)

### Success Criteria
- "sh" → Shortwave appears at top when Shortwave is frequently used (even if Shortcuts is a better string match)
- Per-query frecency: selecting "Shortwave" for "sh" boosts it specifically for "sh" queries
- Existing frecency data continues to work (migration preserves global frecency)
- No pinning feature needed — frecency handles favorite apps naturally

---

## Improvement 3: Better Fuzzy Matching (Priority: Medium)

### Problem
No explicit word-boundary/acronym matching. nucleo handles this somewhat but doesn't give explicit bonuses for matching initial letters of words.

### Requirements

#### 3.1 Word-Boundary/Acronym Bonus Scoring
- Add explicit bonus scoring when query characters match word boundaries in the target
- Word boundaries: start of string, characters after spaces, hyphens, underscores, camelCase transitions
- Examples that should get bonuses:
  - "ss" → **S**ystem **S**ettings (both chars match word starts)
  - "vsc" → **V**isual **S**tudio **C**ode (all chars match word starts)
  - "gc" → **G**oogle **C**hrome
- The bonus should be additive on top of nucleo's base match score
- Keep existing spread factor at 1.5x (no change)

### Technical Approach
- **Fuzzy matching changes** (`photoncast-core/src/search/fuzzy.rs`):
  - After nucleo produces a match score, analyze match positions against word boundaries
  - Calculate acronym bonus: count how many matched characters fall on word boundaries
  - Apply bonus proportional to the ratio of boundary matches (e.g., 2/2 boundary matches for "ss" → System Settings = full bonus)
  - Bonus should be significant enough to promote acronym matches but not override strong frecency
- Keep nucleo as the primary matcher (no replacement)
- Keep existing prefix bonus (+50%) and spread factor (1.5x)

### Success Criteria
- "ss" matches System Settings with higher score than without boundary bonus
- "vsc" matches Visual Studio Code effectively
- Acronym-style queries feel natural and predictable
- No regression in existing fuzzy match quality

---

## Improvement 4: Faster App Quitting (Priority: Lower)

### Problem
`quit_app_by_bundle_id()` sends terminate and polls every 100ms for up to 5 seconds, blocking the main thread. This causes perceived latency and frozen UI.

### Requirements

#### 4.1 Fire-and-Forget Quit
- Send `NSRunningApplication.terminate()` and **immediately** return without waiting
- Remove the polling loop that waits up to 5 seconds for the app to die
- Dismiss the launcher immediately after sending the terminate signal
- No visual feedback (toast, status indicator) — silent dismiss

#### 4.2 No Async Monitoring
- Do not spawn a background task to monitor quit completion
- Do not show notifications about quit success/failure
- Trust that `terminate()` works; if the app shows a "Save changes?" dialog, that's expected macOS behavior

### Technical Approach
- **Process changes** (`photoncast-apps/src/process.rs`):
  - Modify `quit_app_by_bundle_id()` to send terminate and return immediately
  - Remove the `loop { sleep(100ms); check_terminated; if elapsed > 5s break }` pattern
  - Function becomes: find app by bundle ID → call `terminate()` → return Ok(true)
- **Action handler changes** (`photoncast/src/launcher/actions.rs`):
  - Quit action: call quit → immediately call `self.hide(cx)` (already the flow, just faster now)
  - No need for async spawn or callback

### Success Criteria
- Quitting an app through PhotonCast feels instant (no perceptible delay)
- Launcher dismisses immediately after quit action
- No UI freeze or jank
- Apps still quit correctly (terminate signal is still sent)
- Force quit (`forceTerminate()`) path remains unchanged

---

## Technical Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Per-query frecency increases DB size | Storage growth over time | Prune old entries (>30 days unused), limit tracked prefix lengths to 1-4 chars |
| High frecency multiplier over-boosts rarely-matched apps | Poor results for rare queries | Per-query frecency ensures boosts are query-specific, not just global |
| LSUIElement check adds indexing latency | Slower app scanning | Only applies to CoreServices path, minimal overhead |
| Fire-and-forget quit misses "Save changes?" dialogs | User thinks quit worked | Expected macOS behavior — user will see the dialog when switching to the app |
| Schema migration for per-query frecency | Data loss risk | Additive migration (new table), no changes to existing tables |

## Implementation Order

1. **Smarter app indexing** — fixes bad data at the source, cleanest standalone change
2. **Frecency-based sorting** — biggest UX impact, depends on clean index data
3. **Better fuzzy matching** — refines search quality, works with improved frecency
4. **Faster app quitting** — independent quality-of-life fix, can be done anytime
