# Acceptance Matrix — Rust Performance Tuning Enhancements

Date: 2026-02-09

## AC-1.x Prefetch Throttle

- AC-1.1 — Pass
  - Evidence: `search::spotlight::prefetch` tests passing.
- AC-1.2 — Pass
  - Evidence: epoch-relative timing + `saturating_sub` in `prefetch.rs`.
- AC-1.3 — Pass
  - Evidence: `search.prefetch.throttled` log/trace path present.

## AC-2.x Async Search

- AC-2.1 — Partial
  - Evidence: debounced scheduling implemented; bounded-call benchmark not yet captured.
- AC-2.2 — Pass
  - Evidence: calculator/calendar path preserved in launcher search flow.
- AC-2.3 — Pending measurement
  - Evidence required: baseline-vs-post p95 latency artifact.
- AC-2.4 — Pass
  - Evidence: generation + cancellation gating and stale-result drop checks.

## AC-3.x Clone Reduction

- AC-3.1 — Pass
  - Evidence: Arc-backed caches in quicklinks/custom commands providers.
- AC-3.2 — Pass
  - Evidence: files provider no longer deep-clones spotlight vectors in hot path.
- AC-3.3 — Partial
  - Evidence: provider benchmark groups exist and build; numeric deltas not captured.

## AC-4.x Watcher Backpressure

- AC-4.1 — Pass
  - Evidence: bounded watcher channel capacities implemented.
- AC-4.2 — Partial
  - Evidence: burst/capacity tests exist; explicit RSS delta artifact pending.
- AC-4.3 — Pass
  - Evidence: dropped event warnings with `dropped_count` field.

## AC-5.x File-Search Consolidation

- AC-5.1 — Partial
  - Evidence: `FileSearchBackend` added and adopted in `FileProvider` + launcher helper.
  - Remaining: additional direct Spotlight/walkdir usages still exist in broader codebase.
- AC-5.2 — Pass
  - Evidence: fallback only for timeout/zero results; non-empty spotlight results do not fallback.
- AC-5.3 — Pass
  - Evidence: structured fallback reason logs in backend.
- AC-5.4 — Pass
  - Evidence: targeted file provider/backend tests pass.

## AC-6.x Observability

- AC-6.1 — Partial
  - Evidence: spans/traces in engine, provider paths, prefetch, watcher, backend.
  - Remaining: not every listed subsystem has complete elapsed-field parity.
- AC-6.2 — Partial
  - Evidence: `cargo bench -p photoncast-core --no-run` passes.
  - Remaining: full benchmark run and captured output artifacts.
- AC-6.3 — Pass
  - Evidence: `cargo build -p photoncast-core --features perf` passes.
- AC-6.4 — Partial
  - Evidence: schema fields used broadly; full conformance audit pending.

## AC-7.x Cross-Cutting

- AC-7.1 — Pass
  - Evidence: `cargo clippy --workspace -- -D warnings` passes.
- AC-7.2 — Pending
  - Evidence: `cargo fmt --check` currently fails due unrelated pre-existing formatting diffs.
- AC-7.3 — Pass
  - Evidence: `cargo test --workspace` passes.
- AC-7.4 — Partial
  - Evidence: no new panic-driven flow introduced in touched runtime paths; full repo audit pending.
- AC-7.5 — Pass
  - Evidence: only optional `tracing-flame` + optional `tracing-subscriber` wiring for `perf` feature.
