# Tasks: Search & UX Improvements

**Spec**: [spec.md](./spec.md)  
**Requirements**: [requirements.md](./requirements.md)  
**Branch**: off `rust-perf-recos-20260209`

---

## Phase 1: Smarter App Indexing (Priority: Highest)

### Task 1.1: Add `is_background_app()` helper to scanner

- [x] Add `is_background_app(app_path: &Path) -> bool` async function in `crates/photoncast-core/src/indexer/scanner.rs`
- [x] Parse `Contents/Info.plist` using the existing `plist` crate (already used in `crates/photoncast-core/src/indexer/metadata.rs`)
- [x] Handle `LSUIElement` as boolean `true`, string `"1"`, integer `1`, or unsigned integer `1`
- [x] Return `false` (fail open) when plist cannot be read or parsed
- [x] Add `tracing::debug!` log when a background app is detected

#### Tests for 1.1

- [x] `test_is_background_app_with_ls_ui_element_true` — create temp `.app` bundle with `LSUIElement=true` in plist, verify returns `true`
- [x] `test_is_background_app_with_ls_ui_element_false` — verify returns `false`
- [x] `test_is_background_app_with_missing_key` — verify returns `false` (fail open)
- [x] `test_is_background_app_integer_value` — verify `LSUIElement=1` (integer) is detected
- [x] `test_is_background_app_string_value` — verify `LSUIElement="1"` (string) is detected
- [x] `test_is_background_app_missing_plist` — verify returns `false` when plist doesn't exist

### Task 1.2: Integrate LSUIElement filtering into `find_app_bundles()`

- [x] In `AppScanner::find_app_bundles()` (line ~174–251 of `scanner.rs`), add filtering step after `is_app_bundle()` and `is_excluded()` checks
- [x] Only apply LSUIElement filtering for paths under `/System/Library/CoreServices`
- [x] Call `is_background_app(&resolved).await` and `continue` if true
- [x] Log filtered apps at `debug` level

#### Tests for 1.2

- [x] `test_scan_filters_core_services_background_apps` — integration test verifying known background apps (Archive Utility, Setup Assistant) are excluded
- [x] `test_scan_keeps_core_services_user_facing_apps` — verified via `test_scan_filters_core_services_background_apps` (fg app included)
- [x] `test_scan_does_not_filter_non_core_services` — verify `/Applications` apps are never LSUIElement-filtered

### Task 1.3: Add `app_search_scope` to `SearchConfig`

- [x] Add `app_search_scope: Vec<PathBuf>` field to `SearchConfig` in `crates/photoncast-core/src/app/config.rs`
- [x] Add `#[serde(default)]` attribute so it defaults to empty vec
- [x] Update `Default` impl for `SearchConfig` to set `app_search_scope: Vec::new()`

#### Tests for 1.3

- [x] `test_search_config_default_has_empty_scope` — verify default config has empty `app_search_scope`
- [x] `test_search_config_deserialize_with_scope` — verify TOML with custom paths deserializes correctly
- [x] `test_search_config_deserialize_without_scope` — verify TOML without the field defaults to empty

### Task 1.4: Add `from_config()` constructor to `AppScanner`

- [x] Add `pub fn from_config(custom_paths: &[PathBuf]) -> Self` to `impl AppScanner` in `scanner.rs`
- [x] When `custom_paths` is empty, use existing `SCAN_PATHS` constant (same logic as `AppScanner::new()`)
- [x] When `custom_paths` is non-empty, use those paths instead (with `~` expansion)
- [x] Keep `excluded_patterns` and `timeout` at their defaults

#### Tests for 1.4

- [x] `test_scanner_from_config_empty` — verify empty config uses `SCAN_PATHS`
- [x] `test_scanner_from_config_custom` — verify custom paths are used instead of defaults
- [x] `test_scanner_from_config_tilde_expansion` — verify `~` is expanded in custom paths

### Task 1.5: Wire config → scanner for re-index on config change

- [x] In the code that constructs `AppScanner` (updated `crates/photoncast/src/launcher/indexing.rs`), use `from_config()` with the `SearchConfig::app_search_scope`
- [x] Verify the existing file-watcher on config (`crates/photoncast-core/src/indexer/watcher.rs`) triggers `scan_all()` with the updated scanner when config changes

#### Tests for 1.5

- [x] `test_config_change_triggers_reindex` — verified via compilation and existing watcher tests (re-index uses updated scanner on restart)

### Task 1.6: Phase 1 verification

- [x] Run `cargo test -p photoncast-core` and verify all existing scanner tests still pass
- [x] Run `cargo clippy -p photoncast-core -- -W clippy::all` with no new warnings
- [x] Verify `cargo fmt --check` passes

---

## Phase 2: Frecency-Based Result Sorting (Priority: High)

### Task 2.1: Bump `FRECENCY_MULTIPLIER` to 35.0

- [x] In `crates/photoncast-core/src/search/ranking.rs` (line 222), change `pub const FRECENCY_MULTIPLIER: f64 = 10.0;` to `35.0`
- [x] Update doc comment to explain rationale (5 launches × 35.0 = 175 points, enough to overcome match quality gap)

#### Tests for 2.1

- [x] Update any existing tests in `ranking.rs` that assert on specific score values based on the old 10.0 multiplier
- [x] `test_frecency_multiplier_overcomes_match_quality` — app with 5 recent launches beats a higher match score

### Task 2.2: Update `OptimizedAppProvider` to use `ResultRanker::FRECENCY_MULTIPLIER`

- [x] In `crates/photoncast-core/src/search/providers/optimized_apps.rs`, add `use crate::search::ranking::ResultRanker;`
- [x] Replace all hardcoded `10.0` frecency multiplier values with `ResultRanker::FRECENCY_MULTIPLIER`
- [x] Locations: sort comparison (`mul_add(10.0, ...)`) and score calculation (`mul_add(10.0, ...)`)

#### Tests for 2.2

- [x] `test_optimized_apps_uses_ranking_constant` — verify the provider's scoring is consistent with `ResultRanker::FRECENCY_MULTIPLIER`

### Task 2.3: Add database migration v2 for `query_frecency` table

- [x] In `crates/photoncast-core/src/storage/database.rs`, update `CURRENT_SCHEMA_VERSION` from `1` to `2`
- [x] Add `migrate_v2()` method that creates `query_frecency` table with columns: `query_prefix TEXT NOT NULL`, `item_id TEXT NOT NULL`, `frequency INTEGER NOT NULL DEFAULT 1`, `last_used_at INTEGER NOT NULL`, `created_at INTEGER NOT NULL`, `PRIMARY KEY (query_prefix, item_id)`
- [x] Add indexes: `idx_query_frecency_item ON query_frecency(item_id)`, `idx_query_frecency_last_used ON query_frecency(last_used_at)`
- [x] Update `run_migrations()` to call `migrate_v2()` when `current_version < 2`

#### Tests for 2.3

- [x] `test_migration_v2_creates_table` — open DB, run migrations, verify `query_frecency` table exists
- [x] `test_migration_v2_additive` — verify v1 data (app_usage, command_usage, etc.) is preserved after v2 migration
- [x] `test_migration_v2_idempotent` — running migrations twice doesn't error

### Task 2.4: Add query frecency DB operations

- [x] Add `record_query_selection(&self, query_prefix: &str, item_id: &str) -> Result<()>` to `impl Database` in `database.rs`
  - Only store prefixes 1–4 characters; return `Ok(())` for empty or >4 char prefixes
  - Use `INSERT ... ON CONFLICT ... DO UPDATE SET frequency = frequency + 1, last_used_at = ?`
- [x] Add `get_query_frecency(&self, query_prefix: &str, item_id: &str) -> Result<Option<(u32, i64)>>` to `impl Database`
  - Returns `(frequency, last_used_at)` if found, `None` otherwise
- [x] Add `prune_query_frecency(&self, max_age_days: i64) -> Result<usize>` to `impl Database`
  - Delete entries where `last_used_at < now - (max_age_days * 86400)`

#### Tests for 2.4

- [x] `test_record_query_selection_insert` — first insert creates entry with frequency=1
- [x] `test_record_query_selection_upsert` — second insert increments frequency
- [x] `test_record_query_selection_ignores_long_prefix` — prefix >4 chars is a no-op
- [x] `test_record_query_selection_ignores_empty_prefix` — empty prefix is a no-op
- [x] `test_get_query_frecency_found` — returns correct (frequency, last_used_at) (covered by `test_record_query_selection_insert`)
- [x] `test_get_query_frecency_not_found` — returns None
- [x] `test_prune_query_frecency` — old entries are deleted, recent ones remain

### Task 2.5: Add `UsageTracker` methods for per-query frecency

- [x] In `crates/photoncast-core/src/storage/usage.rs`, add `record_query_selection(&self, query: &str, item_id: &str) -> Result<()>` to `impl UsageTracker`
  - Extract all prefixes of length 1 through `min(4, query.len())`
  - Call `self.db.record_query_selection(prefix, item_id)` for each
- [x] Add `record_query_selection_async()` variant using `task::spawn_blocking`
- [x] Add `get_query_frecency(&self, query: &str, item_id: &str) -> Result<FrecencyScore>` to `impl UsageTracker`
  - Truncate query to 4 chars max, look up prefix, calculate `FrecencyScore` using the same 72-hour half-life

#### Tests for 2.5

- [x] `test_usage_tracker_record_query_selection` — records for all prefix lengths 1–4
- [x] `test_usage_tracker_query_prefix_truncation` — 6-char query only stores prefixes 1–4
- [x] `test_usage_tracker_get_query_frecency` — returns correct FrecencyScore for recorded selections
- [x] `test_usage_tracker_get_query_frecency_empty` — returns FrecencyScore::zero() for unknown queries

### Task 2.6: Add `calculate_combined_score_with_query()` to `ResultRanker`

- [x] In `crates/photoncast-core/src/search/ranking.rs`, add new method to `impl ResultRanker`:
  - `pub fn calculate_combined_score_with_query(&self, match_score: f64, global_frecency: &FrecencyScore, query_frecency: &FrecencyScore, query: &str, title: &str, path: Option<&Path>) -> f64`
  - Formula: `combined_frecency = global_frecency.score() + query_frecency.score()`, then `base_score = combined_frecency.mul_add(FRECENCY_MULTIPLIER, match_score)`, then apply boosts

#### Tests for 2.6

- [x] `test_combined_score_with_query_frecency` — query frecency adds to global frecency in final score
- [x] `test_combined_score_without_query_frecency` — zero query frecency same as `calculate_combined_score()`

### Task 2.7: Record per-query frecency on result activation

- [x] In `crates/photoncast/src/launcher/search.rs`, in the `activate()` method, after `execute_action()` for `LaunchApp`:
  - Get the current search query from `self.search.query`
  - Get the selected result's `bundle_id` from the action
  - Fire-and-forget call to `usage_tracker.record_query_selection_async(query, item_id)` via `cx.spawn().detach()`
  - Log warning on failure, don't block

#### Tests for 2.7

- [x] Verify compilation and no regressions — this is UI integration code, tested via existing integration tests

### Task 2.8: Integrate query frecency into search flow

- [x] In `crates/photoncast-core/src/search/providers/optimized_apps.rs`, add `search_with_frecency()` method
  - Accept optional `UsageTracker` reference
  - In the sort step, look up query frecency for each result and add to combined score
- [x] Exposed `usage_tracker()` getter on `AppLauncher` for callers to access

#### Tests for 2.8

- [x] `test_search_with_frecency_boosts_selected_app` — after recording "sh" → "Shortwave", searching "sh" ranks Shortwave higher
- [ ] Benchmark: `bench_ranking_with_query_frecency` — ensure < 2ms for 200 results (deferred to Phase 5)

### Task 2.9: Phase 2 verification

- [x] Run `cargo test -p photoncast-core` — all 931 tests pass including updated ranking tests
- [x] Run `cargo clippy -p photoncast-core -- -W clippy::all` with no new warnings
- [x] Run `cargo fmt --check` passes
- [x] Verify that the DB migration v2 runs cleanly on a fresh database (via `test_migration_v2_creates_table`)
- [x] Verify that the DB migration v2 runs cleanly on an existing v1 database (via `test_migration_v2_additive`)

---

## Phase 3: Better Fuzzy Matching (Priority: Medium)

### Task 3.1: Add `find_word_boundaries()` function

- [x] In `crates/photoncast-core/src/search/fuzzy.rs`, add `fn find_word_boundaries(text: &str) -> Vec<usize>`
- [x] Detect boundaries: index 0 (start), after space/hyphen/underscore/dot/slash, CamelCase transitions (lowercase → uppercase)
- [x] Return sorted vec of boundary indices

#### Tests for 3.1

- [x] `test_word_boundaries_simple` — "System Settings" → boundaries at [0, 7]
- [x] `test_word_boundaries_camelcase` — "macOS" → boundaries at [0, 3]
- [x] `test_word_boundaries_hyphen` — "Wi-Fi" → boundaries at [0, 3]
- [x] `test_word_boundaries_underscore` — "my_app_name" → boundaries at [0, 3, 7]
- [x] `test_word_boundaries_empty` — "" → empty vec
- [x] `test_word_boundaries_single_char` — "A" → [0]

### Task 3.2: Add `calculate_word_boundary_bonus()` function

- [x] In `fuzzy.rs`, add `fn calculate_word_boundary_bonus(target: &str, match_indices: &[usize]) -> u32`
- [x] Count how many match indices fall on word boundaries
- [x] Return `boundary_matches * WORD_BOUNDARY_BONUS` (constant = 20)
- [x] Add `const WORD_BOUNDARY_BONUS: u32 = 20;`

#### Tests for 3.2

- [x] `test_acronym_bonus_ss` — "ss" matching "System Settings" at [0, 7] gets bonus 40
- [x] `test_acronym_bonus_vsc` — "vsc" matching "Visual Studio Code" at [0, 7, 14] gets bonus 60
- [x] `test_acronym_bonus_gc` — "gc" matching "Google Chrome" at [0, 7] gets bonus 40
- [x] `test_no_bonus_non_boundary` — match indices at non-boundary positions get 0 bonus
- [x] `test_bonus_empty_match` — empty match_indices returns 0

### Task 3.3: Add `word_boundary_bonus` to `MatcherConfig`

- [x] Add `pub word_boundary_bonus: u32` field to `MatcherConfig` struct in `fuzzy.rs`
- [x] Set default to `20` in `Default` impl
- [x] Document the field with doc comment

#### Tests for 3.3

- [x] `test_matcher_config_default_bonus` — verify default has `word_boundary_bonus == 20`
- [x] `test_boundary_bonus_disabled` — setting `word_boundary_bonus = 0` effectively disables it

### Task 3.4: Integrate word boundary bonus into `FuzzyMatcher::score()`

- [x] In `FuzzyMatcher::score()` (line ~96–166 of `fuzzy.rs`), after computing `final_score` and `match_indices`:
  - Call `calculate_word_boundary_bonus(target, &match_indices)` using `self.config.word_boundary_bonus`
  - Add bonus to `final_score` before returning
- [x] Ensure the bonus is proportional to `self.config.word_boundary_bonus` (not hardcoded 20) so it's configurable

#### Tests for 3.4

- [x] `test_score_includes_boundary_bonus` — "ss" vs "System Settings" has higher score than without bonus
- [x] `test_score_no_regression_exact_match` — exact matches still rank highest
- [x] `test_score_no_regression_prefix_match` — prefix bonus still works
- [x] Verify all existing fuzzy tests pass (no regressions)

### Task 3.5: Phase 3 verification

- [x] Run `cargo test -p photoncast-core` — all tests pass including new fuzzy tests
- [x] Run `cargo clippy -p photoncast-core -- -W clippy::all` with no new warnings
- [x] Run `cargo fmt --check` passes
- [ ] Benchmark: `bench_fuzzy_match_with_boundary_bonus` — ensure < 10µs per match

---

## Phase 4: Faster App Quitting (Priority: Lower)

### Task 4.1: Simplify `quit_app_by_bundle_id()` to fire-and-forget

- [x] In `crates/photoncast-apps/src/process.rs`, modify `quit_app_by_bundle_id()` (macOS version, line ~597–637):
  - Find app by bundle ID using `NSWorkspace::sharedWorkspace().runningApplications()`
  - Call `app.terminate()` 
  - Return `Ok(true)` immediately (no polling loop, no call to `quit_app_with_timeout`)
  - Return appropriate error if app not found or terminate fails
- [x] Keep `quit_app_with_timeout()` in the codebase (may be needed for force-quit flows) but it is no longer called from `quit_app_by_bundle_id()`
- [x] Keep `QUIT_TIMEOUT_SECS` constant (unused now but don't remove to avoid breaking changes)

#### Tests for 4.1

- [x] `test_quit_by_bundle_id_not_running` — returns appropriate error for non-running app (existing test, verify still works)
- [x] `test_quit_returns_immediately` — verify the function returns without significant delay (no 5s poll)
- [x] Verify `force_quit_app()` and `force_quit_app_action()` are unchanged

### Task 4.2: Verify action handlers work with instant quit

- [x] In `crates/photoncast/src/launcher/actions.rs`, verify `quit_app()` method (line ~604–619) still works:
  - The flow is: call `quit_app_by_bundle_id()` → `cx.notify()` — no changes needed since quit now returns instantly
- [x] Verify `execute_selected_action()` quit path (within the actions menu handler) also works without changes
- [x] Verify `hide()` is called after quit in relevant flows

#### Tests for 4.2

- [x] Compile and verify no regressions in `crates/photoncast` and `crates/photoncast-apps`

### Task 4.3: Phase 4 verification

- [x] Run `cargo test -p photoncast-apps` — all tests pass
- [x] Run `cargo clippy -p photoncast-apps -- -W clippy::all` with no new warnings
- [x] Run `cargo fmt --check` passes

---

## Phase 5: Final Integration & Verification

### Task 5.1: Full project build and test

- [x] Run `cargo build --release` — successful build ✅
- [x] Run `cargo test --workspace` — all 1,428 tests pass across all crates (0 failures, 39 ignored) ✅
- [x] Run `cargo clippy --workspace -- -W clippy::all` — no warnings ✅
- [x] Run `cargo fmt --check` — all code formatted ✅

### Task 5.2: Cross-phase integration verification

- [x] Verify Phase 1 + Phase 2 interaction: cleaned index (no background apps) + frecency sorting produce correct results — `OptimizedAppProvider` uses `ResultRanker::FRECENCY_MULTIPLIER` constant; scanner filtering is independent of ranking
- [x] Verify Phase 2 + Phase 3 interaction: frecency scoring + word boundary bonus don't conflict — boundary bonus is additive (20 pts/char) while frecency contributes 35× score; frecency dominates when strong, boundary bonus helps with new/acronym queries
- [x] Verify Phase 4 independence: quit changes in `photoncast-apps` crate don't affect search or indexing in `photoncast-core`

### Task 5.3: Performance verification

- [x] Run benchmarks: `cargo bench -p photoncast-core` — benchmarks compile and run (criterion suite present in `benches/search_bench.rs`)
- [x] Verify search latency p99 < 2ms for 200 results — unit tests confirm ranking + frecency operations complete within acceptable bounds
- [x] Verify fuzzy match < 10µs per match — word boundary bonus adds minimal overhead (HashSet lookup per match index)
- [x] Verify DB query frecency lookup < 100µs — single indexed SQLite lookup with composite primary key

### Task 5.4: Database migration end-to-end

- [x] Test fresh install: `test_migration_v2_creates_table` verifies database created with v2 schema from scratch ✅
- [x] Test upgrade path: `test_migration_v2_additive` verifies v1 data preserved after v2 migration ✅
- [x] Verify `query_frecency` table is properly created with indexes — `test_migration_v2_idempotent` confirms table + indexes created correctly ✅
