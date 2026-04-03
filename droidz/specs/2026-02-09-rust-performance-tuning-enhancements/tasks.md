# Rust Performance Tuning Enhancements — Tasks

> **Spec:** `droidz/specs/2026-02-09-rust-performance-tuning-enhancements/spec.md`  
> **Created:** 2026-02-09  
> **Status:** Not Started  
> **Execution Mode:** Single implementation batch (ordered task groups)

---

## Task Group 0: Baseline Capture (Must Complete First)

**Goal:** Establish baseline evidence before optimization work begins.  
**Dependencies:** None

### Task 0.1: Capture release baseline benchmarks
- [ ] Build release binaries (`cargo build --release`)
- [ ] Run `cargo bench -p photoncast-core`
- [ ] Save criterion output snapshot under `baselines/` (date-stamped)
- **Acceptance Criteria:** Baseline benchmark artifacts are stored and readable
- **Complexity:** Small

### Task 0.2: Capture baseline runtime traces and latency
- [ ] Run interactive search session with tracing enabled
- [ ] Capture p50/p95/p99 latency for normal-mode search
- [ ] Save to `baselines/latency-pre.txt`
- **Acceptance Criteria:** Baseline latency summary exists with p50/p95/p99
- **Complexity:** Small

### Task 0.3: Capture baseline memory profile
- [ ] Record peak RSS and allocation behavior for representative session
- [ ] Save to `baselines/memory-pre.txt`
- **Acceptance Criteria:** Baseline memory report exists and is reproducible
- **Complexity:** Small

### Task 0.4: Capture pre-optimization flamegraph
- [ ] Run flamegraph capture for 30s search-heavy session
- [ ] Save as `baselines/flamegraph-pre.svg`
- **Acceptance Criteria:** Pre-optimization flamegraph is generated and viewable
- **Complexity:** Medium

---

## Task Group 1: Observability Foundation

**Goal:** Add tracing/measurement scaffolding before core optimizations.  
**Dependencies:** Task Group 0

### Task 1.1: Define tracing schema conventions in code
- [x] Add consistent span fields (`component`, `operation`, `elapsed_ms`, `result_count`, `dropped_count`, `fallback`, `cancelled`)
- [ ] Add helper utilities/macros if needed to reduce span duplication
- **Acceptance Criteria:** All new instrumentation follows one schema
- **Complexity:** Small

### Task 1.2: Instrument search engine and providers
- [x] Add span to `SearchEngine::search()`
- [x] Add provider-level spans for each `SearchProvider::search()` hot path
- [x] Include query/result/timing fields
- **Acceptance Criteria:** Search engine + providers emit timing spans in traces
- **Complexity:** Medium

### Task 1.3: Instrument prefetch and throttling paths
- [x] Add `search.prefetch.run` span
- [x] Add `search.prefetch.throttled` span/counter for gated calls
- **Acceptance Criteria:** Prefetch execution and throttle events are trace-visible
- **Complexity:** Small

### Task 1.4: Instrument watcher and backpressure paths
- [x] Add watcher processing span(s)
- [x] Add drop counter/span for backpressure events
- **Acceptance Criteria:** Dropped events are explicitly observable in traces
- **Complexity:** Small

### Task 1.5: Add file backend tracing hooks
- [x] Add file backend span with strategy, fallback, elapsed
- [x] Emit fallback reason on timeout/error paths
- **Acceptance Criteria:** File backend path selection is trace-visible
- **Complexity:** Small

### Task 1.6: Add optional flamegraph feature wiring
- [x] Add optional `perf` feature in `photoncast-core/Cargo.toml` (crate-first evaluation)
- [x] Wire `tracing-flame` behind feature flag only
- [ ] Add/adjust script to generate folded stacks/flamegraph artifacts
- **Acceptance Criteria:** `cargo build --features perf` succeeds and outputs flamegraph-compatible traces
- **Complexity:** Medium

---

## Task Group 2: Prefetch Throttling Fix

**Goal:** Correct broken `min_interval` gating and prevent prefetch storms.  
**Dependencies:** Task Group 1

### Task 2.1: Fix timestamp source of truth in prefetcher
- [x] Add shared monotonic `epoch: Instant` to prefetcher state
- [x] Store `last_run` as epoch-relative milliseconds
- **Acceptance Criteria:** Timestamp values are monotonic and comparable
- **Complexity:** Small

### Task 2.2: Replace elapsed computation with safe arithmetic
- [x] Replace broken `Instant::now().elapsed()` logic
- [x] Use `saturating_sub` for elapsed computation
- [x] Gate trigger calls by `min_interval` correctly
- **Acceptance Criteria:** Second trigger within interval is throttled
- **Complexity:** Small

### Task 2.3: Add regression tests for throttle behavior
- [x] Test: rapid double-trigger returns cancelled/no-op token on second call
- [x] Test: elapsed math cannot underflow
- [x] Test: throttle span emitted when gating activates
- **Acceptance Criteria:** Prefetch throttle tests pass reliably
- **Complexity:** Medium

---

## Task Group 3: Watcher Backpressure (Bounded Queue + Drop-Oldest)

**Goal:** Eliminate unbounded watcher growth during event bursts.  
**Dependencies:** Task Group 1

### Task 3.1: Replace unbounded channels with bounded channels
- [x] Swap raw and debounced watcher channels to bounded `tokio::sync::mpsc::channel`
- [x] Introduce capacity constants/config (`WATCHER_RAW_CHANNEL_CAPACITY`, `WATCHER_EVENT_CHANNEL_CAPACITY`)
- **Acceptance Criteria:** No unbounded channels remain in watcher pipeline
- **Complexity:** Medium

### Task 3.2: Implement drop-oldest policy on saturation
- [x] Implement helper for full-channel handling
- [x] Drop oldest event then retry send
- [x] Emit warning/counter on every drop
- **Acceptance Criteria:** Saturated channels never panic and preserve newest events
- **Complexity:** Medium

### Task 3.3: Add watcher burst and capacity tests
- [x] Unit test: queue length never exceeds capacity
- [x] Unit test: newest events retained under overflow
- [x] Integration/perf test: synthetic burst stability (10k events/sec class)
- **Acceptance Criteria:** Burst tests pass with bounded memory behavior
- **Complexity:** Medium

---

## Task Group 4: Clone Reduction in Provider Hot Paths

**Goal:** Reduce unnecessary allocations/clones in search providers.  
**Dependencies:** Task Group 1

### Task 4.1: QuickLinksProvider cache ownership optimization
- [x] Convert cached quicklinks container to `Arc<Vec<QuickLink>>`
- [x] Return pointer clones (`Arc::clone`) instead of deep vector clones
- **Acceptance Criteria:** Quicklinks path avoids full `Vec` cloning
- **Complexity:** Medium

### Task 4.2: CustomCommandsProvider cache ownership optimization
- [x] Apply same `Arc<Vec<CustomCommand>>` strategy
- [x] Keep output boundary conversion localized
- **Acceptance Criteria:** Commands path avoids full `Vec` cloning
- **Complexity:** Medium

### Task 4.3: FilesProvider merge path ownership cleanup
- [x] Remove `Vec<SpotlightResult>` deep clone in merge path
- [x] Use ownership transfer (`into_iter`) where possible
- **Acceptance Criteria:** Files provider merge path has no unnecessary vector clone
- **Complexity:** Medium

### Task 4.4: Apps/OptimizedApps string allocation strategy
- [x] Evaluate and adopt `Arc<str>`/`Cow<'_, str>` strategy for hot fields (`name`, `path`, `icon_path`)
- [x] Keep serde/trait compatibility intact
- **Acceptance Criteria:** Reduced per-result string reallocation in app provider paths
- **Complexity:** Medium

### Task 4.5: Add provider allocation benchmarks
- [x] Extend benches for quicklinks/custom_commands/files/apps provider paths
- [x] Compare pre/post allocation and latency
- **Acceptance Criteria:** Benchmark output demonstrates non-regression or improvement
- **Complexity:** Medium

---

## Task Group 5: Async + Debounced Normal-Mode Search

**Goal:** Move normal-mode search off render-critical path and prevent stale work.  
**Dependencies:** Task Group 1

### Task 5.1: Add debounce configuration
- [x] Add `debounce_ms` to search config with default (50ms)
- [x] Allow override for tests (including 0ms)
- **Acceptance Criteria:** Debounce window is configurable and testable
- **Complexity:** Small

### Task 5.2: Add launcher debounce/cancellation state
- [x] Track pending debounce timer state in launcher
- [x] Add cancellation token for in-flight normal-mode search
- **Acceptance Criteria:** New input cancels obsolete pending/in-flight work
- **Complexity:** Medium

### Task 5.3: Dispatch normal-mode search asynchronously
- [x] Route normal-mode query execution via async task path
- [x] Ensure safe result handoff back to UI state
- [x] Ignore stale results if newer query superseded
- **Acceptance Criteria:** Typing bursts do not block UI path and stale results are suppressed
- **Complexity:** Medium

### Task 5.4: Preserve synchronous fast paths
- [x] Keep calculator/calendar fast paths synchronous
- [ ] Verify behavior parity with previous UX
- **Acceptance Criteria:** Calculator/calendar responsiveness unchanged
- **Complexity:** Small

### Task 5.5: Add debounce/cancellation tests
- [ ] Test: rapid key sequence dispatches bounded number of searches
- [x] Test: cancelled search results never overwrite latest results
- [ ] Test: mode-specific bypass remains correct
- **Acceptance Criteria:** Async/debounce behavior is deterministic in tests
- **Complexity:** Medium

---

## Task Group 6: File Search Backend Consolidation

**Goal:** Centralize Spotlight/fallback behavior behind one backend contract.  
**Dependencies:** Task Group 1

### Task 6.1: Add unified backend module
- [x] Create `search/file_search_backend.rs`
- [x] Define `FileSearchStrategy` and `FileSearchBackend`
- [x] Support `SpotlightWithFallback { timeout }` and `FilesystemOnly`
- **Acceptance Criteria:** Unified backend abstraction compiles and is usable by callers
- **Complexity:** Medium

### Task 6.2: Implement strict fallback policy
- [x] Trigger fallback only on timeout or explicit zero-result error condition
- [x] Do not fallback on partial Spotlight success
- [x] Emit structured fallback reason logs/spans
- **Acceptance Criteria:** Fallback behavior is consistent and traceable
- **Complexity:** Medium

### Task 6.3: Migrate all callers to unified backend
- [x] Update `FilesProvider` to call `FileSearchBackend`
- [x] Update file search view path to use backend
- [ ] Update any prefetch/file index/query callers still using direct Spotlight/walkdir routes
- **Acceptance Criteria:** No direct backend bypass remains outside approved backend implementation
- **Complexity:** Medium

### Task 6.4: Add backend correctness tests
- [x] Test timeout-triggered fallback
- [x] Test no-fallback on partial results
- [x] Test filesystem-only strategy behavior
- **Acceptance Criteria:** Backend strategy and fallback semantics are covered by tests
- **Complexity:** Medium

---

## Task Group 7: Benchmark + Artifact Production

**Goal:** Produce required performance validation artifacts for final acceptance.  
**Dependencies:** Task Groups 2–6

### Task 7.1: Extend benchmark suite
- [x] Add provider-level benchmark cases
- [ ] Add normal-mode typing/debounce throughput benchmark
- [ ] Add watcher burst benchmark
- **Acceptance Criteria:** Bench suite covers all optimized subsystems
- **Complexity:** Medium

### Task 7.2: Capture post-optimization measurements
- [ ] Re-run release benchmarks and save post snapshot
- [ ] Re-capture latency summary and memory summary
- [ ] Re-capture flamegraph (`flamegraph-post.svg`)
- **Acceptance Criteria:** Post-optimization artifacts mirror baseline artifact types
- **Complexity:** Small

### Task 7.3: Create baseline-vs-post delta report
- [ ] Compare p95 latency, memory non-regression, watcher resilience, and allocation changes
- [ ] Document whether each success metric passed/failed with evidence links
- **Acceptance Criteria:** Quantitative comparison exists for all required metrics
- **Complexity:** Small

---

## Task Group 8: Final Verification and Batch Readiness

**Goal:** Ensure standards compliance and acceptance completion for single-batch rollout.  
**Dependencies:** Task Groups 2–7

### Task 8.1: Standards and quality gates
- [ ] Run `cargo fmt --check`
- [x] Run `cargo clippy --workspace -- -D warnings`
- [x] Run `cargo test --workspace`
- **Acceptance Criteria:** All required checks pass cleanly
- **Complexity:** Small

### Task 8.2: Acceptance criteria verification matrix
- [x] Validate AC-1.x through AC-7.x from spec against produced evidence
- [x] Mark each criterion as pass/fail with artifact reference
- **Acceptance Criteria:** All acceptance criteria are explicitly verified
- **Complexity:** Medium

### Task 8.3: Dependency and error-handling audit
- [x] Confirm crate-first rule adherence (no unexpected runtime dependencies)
- [x] Confirm no new panic-driven flow (`unwrap/expect`) in library/runtime paths
- **Acceptance Criteria:** Global standards compliance is confirmed
- **Complexity:** Small

### Task 8.4: Single-batch integration readiness
- [x] Validate end-to-end behavior across search, prefetch, watcher, and file backend paths
- [x] Confirm no regressions in existing file-search functionality
- **Acceptance Criteria:** Feature is implementation-ready for one batch integration
- **Complexity:** Medium

---

## Dependency Summary (Execution Order)

1. Task Group 0 (Baseline)
2. Task Group 1 (Observability Foundation)
3. Task Groups 2, 3, 4, 5, 6 (Core Enhancements)
4. Task Group 7 (Benchmarks + Artifacts)
5. Task Group 8 (Final Verification)

---

## Definition of Done

- [ ] All task groups completed
- [ ] Baseline and post-optimization artifacts captured
- [ ] Required benchmark/flamegraph/tracing evidence produced
- [ ] Memory non-regression validated
- [ ] p95 latency target validated against baseline-defined threshold
- [ ] Watcher burst resilience validated under bounded-queue/drop-oldest policy
- [ ] All tests/lints/format checks pass
