# Rust Performance Tuning Enhancements — Specification

**Date:** 2026-02-09
**Status:** Draft
**Feature Area:** Performance, Optimization, Observability
**Priority:** High
**Rollout:** Single implementation batch

---

## 1. Overview

### 1.1 Purpose

Implement six prioritized performance enhancements identified during the PhotonCast codebase review. These changes target prefetch throttling, UI-path search latency, allocation overhead in provider hot paths, watcher resilience under event bursts, file-search backend consolidation, and runtime observability.

### 1.2 Goals

| # | Goal | Subsystem |
|---|------|-----------|
| 1 | Fix prefetch throttling timestamp logic so `min_interval` gating works correctly | `search/spotlight/prefetch.rs` |
| 2 | Move normal-mode search off the UI thread via async/debounced execution | `launcher/search.rs`, `search/engine.rs` |
| 3 | Reduce clone-heavy provider flows in quicklinks, custom commands, and files providers | `search/providers/{quicklinks,custom_commands,files}.rs` |
| 4 | Add bounded-channel backpressure with drop-oldest policy for the file watcher pipeline | `indexer/watcher.rs` |
| 5 | Consolidate file-search backend behavior and tighten fallback policy | `search/file_index.rs`, `search/file_query.rs`, `spotlight/service.rs` |
| 6 | Add observability metrics and tracing spans for performance validation | Cross-cutting (all subsystems above) |

### 1.3 Non-Goals (Out of Scope)

- New user-facing features or UI changes.
- Changing the search ranking algorithm or frecency model.
- Migrating away from GPUI, Tokio, or `notify` crate.
- Multi-platform support (remains macOS-only).
- Extension system performance (separate spec).
- Database schema changes to `rusqlite` storage.

### 1.4 Success Criteria (Baseline-Driven)

All targets are relative to a measured baseline captured **before** any code changes (see §3).

| Metric | Target |
|--------|--------|
| p95 search latency (normal mode) | ≤ baseline × 0.80 (20% improvement) — refined after baseline |
| Peak memory / allocation rate | Non-regression vs baseline (no increase) |
| Watcher event-burst resilience | Zero panics or unbounded growth under 10 000 events/sec synthetic burst |
| Prefetch throttle accuracy | ≤ 1 prefetch run per `min_interval` window (verified via trace counter) |
| Observability coverage | Every subsystem in §2 emits at least one tracing span with timing |

---

## 2. Architecture Changes by Subsystem

### 2.1 Prefetch Throttling Timestamp Fix

**Current problem:**
In `SpotlightPrefetcher::trigger()`, the throttling guard computes `elapsed_since_last` incorrectly. It subtracts `last_run_ms` (an absolute timestamp stored as milliseconds-since-process-start) from `now.elapsed().as_millis()` — but `now` was just created, so `now.elapsed()` is near-zero, producing an arithmetic underflow or nonsensical value. The net effect is that throttling never gates repeat calls, allowing prefetch storms.

**Required change:**

```rust
// BEFORE (broken):
let now = Instant::now();
let last_run_ms = self.last_run.load(Ordering::SeqCst);
let elapsed_since_last =
    Duration::from_millis(now.elapsed().as_millis() as u64 - last_run_ms);

// AFTER (correct — store epoch-relative millis):
let now_ms = self.epoch.elapsed().as_millis() as u64;   // monotonic epoch ref
let last_run_ms = self.last_run.load(Ordering::SeqCst);
let elapsed_since_last = Duration::from_millis(now_ms.saturating_sub(last_run_ms));
```

Add a shared `epoch: Instant` field (created once in the constructor) and store `epoch.elapsed()` at completion rather than `Instant::now().elapsed()`.

**Files changed:**
- `crates/photoncast-core/src/search/spotlight/prefetch.rs`

**Error handling:** Use `saturating_sub` to prevent underflow; log a `tracing::warn!` if the guard fires (helps debugging).

---

### 2.2 Async / Debounced Normal-Mode Search

**Current problem:**
`LauncherWindow::on_query_change()` invokes `SearchEngine::search()` synchronously on every keystroke. For fast typists this queues redundant search passes that compete for CPU on the GPUI render cycle.

**Required changes:**

1. **Debounce input** — introduce a configurable debounce window (default 50 ms) in the launcher. Use `tokio::time::sleep` (or GPUI timer) to delay dispatching the search until the user pauses typing.

2. **Async search dispatch** — spawn the search engine call on a Tokio `spawn_blocking` (if CPU-bound) or a regular `tokio::spawn` task, then send results back to the GPUI view via an `mpsc` channel or GPUI `cx.spawn`.

3. **Cancel in-flight searches** — on every new keystroke, cancel the previous pending search via a `CancellationToken` (reuse the pattern already in `prefetch.rs`). Only the most recent query's results are applied to the view state.

**Files changed:**
- `crates/photoncast/src/launcher/search.rs` — debounce + async dispatch
- `crates/photoncast/src/launcher/mod.rs` — add debounce timer state, cancellation token
- `crates/photoncast-core/src/search/engine.rs` — accept optional `CancellationToken`, check between provider calls

**Constraints:**
- Preserve existing `SearchMode::Calendar` and calculator fast-paths which must remain synchronous for instant feedback.
- Debounce window must be configurable via `SearchConfig` (for testing at 0 ms).

---

### 2.3 Clone Reduction in Provider Hot Paths

**Current problem:**
Several providers clone entire collections on every search call:
- `QuickLinksProvider` — `return links.clone()` clones the full `Vec<QuickLink>` from cache.
- `CustomCommandsProvider` — `return commands.clone()` clones `Vec<CustomCommand>`.
- `FilesProvider` — `results.clone()` clones spotlight result vectors.
- `AppsProvider` / `OptimizedAppProvider` — clone `name`, `icon_path`, `path` per result.

**Required changes:**

| Provider | Strategy |
|----------|----------|
| `QuickLinksProvider` | Return `Arc<Vec<QuickLink>>` from cache; callers borrow via `Arc::clone` (pointer bump, not deep clone). Only construct `SearchResult` values by borrowing fields (`&str` → `String` only at the final output boundary). |
| `CustomCommandsProvider` | Same `Arc`-wrapping strategy for the command cache. |
| `FilesProvider` | Avoid cloning `Vec<SpotlightResult>` when merging; move results into the merge instead of cloning. Use `into_iter()` where ownership transfers. |
| `AppsProvider` / `OptimizedAppProvider` | Use `Cow<'_, str>` or `Arc<str>` for `name`, `path` inside `IndexedApp` so `SearchResult` construction borrows rather than allocates. Evaluate feasibility — if lifetime constraints make this impractical, use `Arc<str>` (one allocation, cheap clone). |

**Files changed:**
- `crates/photoncast-core/src/search/providers/quicklinks.rs`
- `crates/photoncast-core/src/search/providers/custom_commands.rs`
- `crates/photoncast-core/src/search/providers/files.rs`
- `crates/photoncast-core/src/search/providers/apps.rs`
- `crates/photoncast-core/src/search/providers/optimized_apps.rs`

**Coding style alignment:**
- Per `coding-style.md`: "DON'T: Clone When You Can Borrow" — this change directly follows the standard.
- Per `crate-first.md`: Use `std::sync::Arc` (no new crate needed).

---

### 2.4 Watcher Backpressure — Bounded Queue + Drop-Oldest

**Current problem:**
`AppWatcher::start()` creates `mpsc::unbounded_channel()` for both the raw notify events and the debounced `WatchEvent` output. Under a filesystem event burst (e.g., Homebrew bulk install), memory grows without limit.

**Required changes:**

1. **Replace unbounded channels with bounded channels:**

```rust
// Raw events from notify → debounce task
let (raw_tx, raw_rx) = mpsc::channel::<Event>(WATCHER_RAW_CHANNEL_CAPACITY);  // e.g., 512

// Debounced events → consumer
let (event_tx, event_rx) = mpsc::channel::<WatchEvent>(WATCHER_EVENT_CHANNEL_CAPACITY);  // e.g., 128
```

2. **Drop-oldest policy on raw channel:** When `raw_tx.try_send()` returns `TrySendError::Full`, pop the oldest item from the receiver side (via a ring-buffer wrapper or by using a helper that `recv`s one item then retries `send`). Alternatively, use a `tokio::sync::broadcast` channel with a fixed capacity (which drops the oldest automatically when full) — evaluate which approach is simpler.

   Recommended: wrap the bounded `mpsc` sender in a helper:

   ```rust
   /// Sends `event`, dropping the oldest queued event if the channel is full.
   async fn send_or_drop_oldest(
       tx: &mpsc::Sender<Event>,
       rx: &mut mpsc::Receiver<Event>,
       event: Event,
   ) {
       if tx.try_send(event.clone()).is_err() {
           // Channel full — drop oldest
           let _dropped = rx.try_recv();
           tracing::warn!("Watcher channel full, dropped oldest event");
           let _ = tx.try_send(event);
       }
   }
   ```

3. **Emit a tracing counter** every time an event is dropped so observability captures burst behavior.

**Constants** (configurable via `WatcherConfig`):

```rust
const WATCHER_RAW_CHANNEL_CAPACITY: usize = 512;
const WATCHER_EVENT_CHANNEL_CAPACITY: usize = 128;
```

**Files changed:**
- `crates/photoncast-core/src/indexer/watcher.rs`

---

### 2.5 File-Search Backend Consolidation & Fallback Tightening

**Current problem:**
File-search logic is spread across multiple modules (`file_index.rs`, `file_query.rs`, `spotlight/service.rs`, `spotlight/query.rs`, `providers/files.rs`, `providers/commands.rs`) with duplicated fallback logic: some paths try Spotlight first then fall back to `walkdir`, others skip Spotlight entirely. This makes behavior inconsistent and hard to reason about.

**Required changes:**

1. **Introduce a single `FileSearchBackend` trait** (or enum dispatch) in `search/file_search_backend.rs` that encapsulates:
   - Primary search path (Spotlight `NSMetadataQuery`).
   - Fallback path (filesystem walk via `ignore` / `walkdir` crate).
   - Timeout and cancellation.

   ```rust
   pub enum FileSearchStrategy {
       /// Use Spotlight, fall back to walkdir on timeout or error.
       SpotlightWithFallback { timeout: Duration },
       /// Use filesystem walk only (no Spotlight).
       FilesystemOnly,
   }

   pub struct FileSearchBackend {
       strategy: FileSearchStrategy,
       spotlight: Option<Arc<SpotlightSearchService>>,
       // ...
   }

   impl FileSearchBackend {
       pub async fn search(&self, query: &str, options: &FileSearchOptions)
           -> Result<Vec<FileResult>>
       {
           match &self.strategy {
               FileSearchStrategy::SpotlightWithFallback { timeout } => {
                   match tokio::time::timeout(*timeout, self.spotlight_search(query, options)).await {
                       Ok(Ok(results)) if !results.is_empty() => Ok(results),
                       _ => {
                           tracing::info!("Spotlight unavailable or empty, falling back to fs walk");
                           self.filesystem_search(query, options).await
                       }
                   }
               }
               FileSearchStrategy::FilesystemOnly => {
                   self.filesystem_search(query, options).await
               }
           }
       }
   }
   ```

2. **All callers** (`FilesProvider`, `file_search_view`, prefetch) use `FileSearchBackend` instead of directly invoking Spotlight or walkdir.

3. **Tighten fallback policy:**
   - Fallback triggers only on Spotlight timeout (configurable, default 1 s) or Spotlight returning zero results after explicit error.
   - Log every fallback event at `tracing::info!` level with reason.
   - Do **not** fall back on partial Spotlight results — partial results are returned immediately.

**Files changed:**
- New: `crates/photoncast-core/src/search/file_search_backend.rs`
- Modified: `crates/photoncast-core/src/search/providers/files.rs`
- Modified: `crates/photoncast-core/src/search/file_index.rs`
- Modified: `crates/photoncast-core/src/search/file_query.rs`
- Modified: `crates/photoncast/src/file_search_view/mod.rs`
- Modified: `crates/photoncast-core/src/search/mod.rs` (re-export)

---

### 2.6 Observability Metrics & Tracing

**Current state:**
The codebase uses `tracing` for structured logging but lacks systematic performance spans. There are no latency histograms, no allocation tracking, and no flamegraph-friendly instrumentation.

**Required additions:**

#### 2.6.1 Tracing Spans

Add `#[tracing::instrument]` or manual `tracing::info_span!` to:

| Span name | Location | Fields |
|-----------|----------|--------|
| `search.engine.search` | `SearchEngine::search()` | `query_len`, `provider_count`, `result_count`, `elapsed_ms` |
| `search.provider.{id}` | Each `SearchProvider::search()` impl | `provider_id`, `result_count`, `elapsed_ms` |
| `search.prefetch.run` | `run_prefetch_queries()` | `queries_run`, `results_cached`, `elapsed_ms` |
| `search.prefetch.throttled` | Throttle guard in `trigger()` | `elapsed_since_last_ms`, `min_interval_ms` |
| `watcher.event.process` | Debounce loop body | `event_kind`, `path` |
| `watcher.backpressure.drop` | Drop-oldest path | `dropped_count` (counter) |
| `file_search.backend` | `FileSearchBackend::search()` | `strategy`, `fallback_triggered`, `elapsed_ms` |

#### 2.6.2 Benchmark Harness Extensions

Extend `crates/photoncast-core/benches/search_bench.rs` with:

- **Provider-level benchmarks** for quicklinks, custom commands, and files providers (before/after clone reduction).
- **Debounce throughput benchmark** simulating rapid keystroke sequences (10 keystrokes in 200 ms).
- **Watcher backpressure benchmark** — synthetic 10 000-event burst, measure memory + latency.

#### 2.6.3 Flamegraph Support

Add a `perf` feature flag to `photoncast-core/Cargo.toml`:

```toml
[features]
perf = ["tracing-flame"]
```

When enabled, configure a `FlameLayer` subscriber that outputs a folded-stack file for `inferno` or `flamegraph` post-processing. Document the workflow in a `scripts/flamegraph.sh` helper.

#### 2.6.4 Telemetry Schema (Internal Only)

All spans use the following conventions (for `tracing` structured fields):

| Field | Type | Description |
|-------|------|-------------|
| `component` | `&str` | Subsystem name (e.g., `"search"`, `"watcher"`, `"prefetch"`) |
| `operation` | `&str` | Specific operation (e.g., `"search"`, `"debounce"`, `"fallback"`) |
| `elapsed_ms` | `f64` | Wall-clock duration in milliseconds |
| `result_count` | `usize` | Number of items returned |
| `dropped_count` | `usize` | Items dropped (backpressure) |
| `fallback` | `bool` | Whether fallback path was taken |
| `cancelled` | `bool` | Whether operation was cancelled |

---

## 3. Baseline Measurement Plan

Before writing any optimization code, capture a baseline:

### 3.1 Steps

1. **Build release profile** (`cargo build --release`).
2. **Run existing benchmark suite** (`cargo bench -p photoncast-core`) and save output to `baselines/YYYY-MM-DD-pre.json`.
3. **Capture a flamegraph** of a 30-second interactive session (type 5 queries, open file search twice). Save to `baselines/flamegraph-pre.svg`.
4. **Record p50/p95/p99 search latencies** from the tracing subscriber output (add temporary timing to `SearchEngine::search` if spans don't exist yet).
5. **Record peak RSS** during the session via `time -l` or Instruments.

### 3.2 Baseline Artifacts

| Artifact | Location | Format |
|----------|----------|--------|
| Criterion benchmark output | `target/criterion/` | HTML + JSON |
| Pre-optimization flamegraph | `baselines/flamegraph-pre.svg` | SVG |
| Latency summary | `baselines/latency-pre.txt` | Plain text (p50, p95, p99) |
| Memory summary | `baselines/memory-pre.txt` | Peak RSS, allocations |

### 3.3 Post-Implementation Comparison

After all changes are merged, re-run the same steps and produce a delta report comparing baseline vs post-optimization numbers.

---

## 4. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Debounce window too aggressive → perceived latency increase | Medium | Medium | Make debounce configurable; default 50 ms; fast-paths (calculator, calendar) bypass debounce |
| `Arc<str>` migration breaks `SearchResult` serialization | Low | Medium | `Arc<str>` implements `Serialize`/`Deserialize` via serde; verify in tests |
| Bounded watcher channel drops important events | Low | High | Debounce still coalesces; log dropped events; generous default capacity (512) |
| File-search consolidation introduces regressions in edge-case queries | Medium | Medium | Keep all existing file-search tests green; add integration tests for Spotlight timeout fallback |
| Tracing overhead in hot paths | Low | Low | Use `tracing`'s compile-time filtering (`max_level_*`) for release builds; spans are zero-cost when no subscriber is attached |

---

## 5. Rollout Plan

**Single batch** — all six changes ship together in one PR (or a short-lived feature branch with individual commits per subsystem).

### 5.1 Commit Order (Recommended)

1. **Observability spans** (§2.6) — land instrumentation first so all subsequent changes produce measurable data.
2. **Prefetch throttle fix** (§2.1) — smallest, most isolated change.
3. **Watcher backpressure** (§2.4) — independent of search path.
4. **Clone reduction** (§2.3) — touches providers but not control flow.
5. **Async search path** (§2.2) — most invasive change; benefits from observability already in place.
6. **File-search consolidation** (§2.5) — largest refactor; lands last.

### 5.2 Feature Flags

No runtime feature flags. The `perf` Cargo feature (§2.6.3) is compile-time only and not enabled in release builds by default.

---

## 6. Testing Strategy

### 6.1 Unit Tests

| Area | Test | File |
|------|------|------|
| Prefetch throttle | Verify `trigger()` is gated by `min_interval`: call twice within interval, assert second returns cancelled token | `prefetch.rs` |
| Prefetch throttle | Verify correct elapsed calculation after fix (mock `Instant` or use short interval) | `prefetch.rs` |
| Bounded channel | Send `CAPACITY + 10` events, assert channel never grows past `CAPACITY`, assert 10 warn logs emitted | `watcher.rs` |
| Drop-oldest | Send burst, verify newest events are retained and oldest are dropped | `watcher.rs` |
| Clone reduction (quicklinks) | Assert `Arc::strong_count` increments on search (pointer clone, not deep clone) | `quicklinks.rs` |
| Clone reduction (commands) | Same `Arc` verification | `custom_commands.rs` |
| File-search backend | Mock Spotlight timeout → verify fallback to filesystem walk | `file_search_backend.rs` |
| File-search backend | Mock Spotlight partial results → verify no fallback | `file_search_backend.rs` |
| Debounce | Send 10 queries at 10 ms intervals with 50 ms debounce → assert only 1-2 search calls dispatched | `launcher/search.rs` |
| Cancellation | Start search, cancel mid-flight, assert result is not applied to view state | `launcher/search.rs` |

### 6.2 Integration Tests

- Full search pipeline with debounced input produces correct results.
- File search with Spotlight disabled falls back cleanly (use `FileSearchStrategy::FilesystemOnly`).
- Watcher handles rapid directory creation/deletion without crash.
- Prefetch + throttle under repeated rapid toggling of file-search modal.

### 6.3 Performance / Regression Tests

- Extend `benches/search_bench.rs` with provider-level benchmarks (pre/post clone reduction).
- Add `benches/watcher_bench.rs` for synthetic burst (10 000 events).
- Add `tests/prefetch_perf_test.rs` — verify wall-clock time of prefetch under throttling.
- All criterion benchmarks must show no regression vs baseline (within noise threshold).

### 6.4 Property-Based Tests

Per `testing/test-writing.md`:

```rust
proptest! {
    #[test]
    fn debounce_always_delivers_last_query(
        queries in prop::collection::vec("[a-z]{1,10}", 1..20),
        debounce_ms in 10u64..200
    ) {
        // After debounce settles, the delivered query must be the last one submitted
    }

    #[test]
    fn bounded_channel_never_exceeds_capacity(
        event_count in 1usize..5000,
        capacity in 1usize..1024
    ) {
        // Channel length never exceeds capacity at any observation point
    }
}
```

### 6.5 Test Commands

```bash
# Unit + integration tests
cargo test --workspace

# Benchmarks (compare against baseline)
cargo bench -p photoncast-core

# Clippy (must pass cleanly)
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check
```

---

## 7. Validation & Acceptance Criteria

Each criterion below must be verified with evidence (benchmark output, trace log, or test result).

### 7.1 Prefetch Throttle (§2.1)

- [ ] **AC-1.1:** Calling `trigger()` twice within `min_interval` returns a cancelled token on the second call.
- [ ] **AC-1.2:** Elapsed-since-last calculation uses monotonic epoch reference; no underflow possible (verified by `saturating_sub`).
- [ ] **AC-1.3:** Tracing span `search.prefetch.throttled` is emitted when throttle fires.

### 7.2 Async Search (§2.2)

- [ ] **AC-2.1:** Typing 10 characters at 30 ms intervals triggers ≤ 3 search engine calls (debounce at 50 ms).
- [ ] **AC-2.2:** Calculator and calendar modes bypass debounce and respond synchronously.
- [ ] **AC-2.3:** p95 search latency (measured via tracing span `search.engine.search`) is ≤ baseline × 0.80.
- [ ] **AC-2.4:** In-flight search is cancelled when a new keystroke arrives; stale results never appear.

### 7.3 Clone Reduction (§2.3)

- [ ] **AC-3.1:** QuickLinks and CustomCommands providers cache is `Arc`-wrapped; search path performs `Arc::clone` (not deep clone).
- [ ] **AC-3.2:** Files provider uses `into_iter()` / ownership transfer for result merging — zero `.clone()` calls on `Vec<SpotlightResult>`.
- [ ] **AC-3.3:** Benchmark `search_bench::provider_*` shows allocation count non-regression or improvement vs baseline.

### 7.4 Watcher Backpressure (§2.4)

- [ ] **AC-4.1:** Watcher uses bounded channels with configurable capacity.
- [ ] **AC-4.2:** Under 10 000 events/sec synthetic burst, no panic, no unbounded memory growth; peak RSS ≤ baseline + 10 MB.
- [ ] **AC-4.3:** Dropped events are logged via `tracing::warn!` with `dropped_count` field.

### 7.5 File-Search Consolidation (§2.5)

- [ ] **AC-5.1:** All file-search callers go through `FileSearchBackend` — no direct Spotlight or walkdir calls remain outside the backend.
- [ ] **AC-5.2:** Fallback triggers only on timeout or zero-result error; partial Spotlight results are returned as-is.
- [ ] **AC-5.3:** Every fallback event emits `tracing::info!` with reason string.
- [ ] **AC-5.4:** All existing file-search tests pass without modification (or with minimal assertion updates for new types).

### 7.6 Observability (§2.6)

- [ ] **AC-6.1:** Every subsystem in §2 has at least one tracing span with `elapsed_ms` field.
- [ ] **AC-6.2:** `cargo bench` produces updated criterion output comparable to baseline.
- [ ] **AC-6.3:** `cargo build --features perf` compiles cleanly and produces a flamegraph-compatible folded-stack file.
- [ ] **AC-6.4:** All span field names follow the schema in §2.6.4.

### 7.7 Cross-Cutting

- [ ] **AC-7.1:** `cargo clippy --workspace -- -D warnings` passes.
- [ ] **AC-7.2:** `cargo fmt --check` passes.
- [ ] **AC-7.3:** `cargo test --workspace` passes (all existing + new tests).
- [ ] **AC-7.4:** No new `unwrap()` / `expect()` in library code (per `error-handling.md`).
- [ ] **AC-7.5:** No new external crate dependencies added without evaluation (per `crate-first.md`). `tracing-flame` is the only potential addition — document rationale.

---

## 8. Dependencies & Crate Evaluation

| Crate | Status | Rationale |
|-------|--------|-----------|
| `tracing` | Already in use | Structured logging — extend with spans |
| `tracing-subscriber` | Already in use | Subscriber configuration |
| `tracing-flame` | **New (optional, behind `perf` feature)** | Flamegraph layer for `tracing`. 500K+ downloads, actively maintained, MIT licensed. Only compiled when `perf` feature is enabled — zero impact on release builds. |
| `tokio` | Already in use | Bounded channels, timers, spawn |
| `parking_lot` | Already in use | Mutex/RwLock for cache |
| `criterion` | Already in use (dev) | Benchmarks |
| `proptest` | Already in use (dev) | Property-based tests |

Per `crate-first.md`: no new runtime dependencies. `tracing-flame` is dev/optional only.

---

## 9. References

- [PhotonCast Architecture](../../ARCHITECTURE.md)
- [Coding Style Standard](../../droidz/standards/global/coding-style.md)
- [Error Handling Standard](../../droidz/standards/global/error-handling.md)
- [Crate-First Standard](../../droidz/standards/global/crate-first.md)
- [API/Backend Standard](../../droidz/standards/backend/api.md)
- [Testing Standard](../../droidz/standards/testing/test-writing.md)
- [Tracing crate documentation](https://docs.rs/tracing/latest/tracing/)
- [tracing-flame crate](https://docs.rs/tracing-flame/latest/tracing_flame/)
- [Tokio bounded channels](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.channel.html)

---

*Specification last updated: 2026-02-09*
