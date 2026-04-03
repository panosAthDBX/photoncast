# Raw Idea

Improve PhotonCast performance by implementing prioritized enhancements from review:

1. Fix prefetch throttling timestamp logic.
2. Move normal-mode search off the UI path (async/debounced where appropriate).
3. Reduce clone-heavy provider flows (quicklinks/custom commands/files).
4. Add bounded channel/backpressure for watcher pipelines.
5. Consolidate file-search backend behavior and fallback policy.
6. Add observability/metrics for search/provider/cache/fallback behavior.

## Clarified Scope and Decisions

### In Scope
- All six recommendations from the performance review:
  1. Prefetch throttling timestamp fix.
  2. Async/debounced normal-mode search path.
  3. Clone-reduction in provider hot paths.
  4. Watcher backpressure with bounded channels.
  5. File-search backend consolidation and fallback policy tightening.
  6. Observability additions for measurement and validation.

### Rollout
- Single implementation batch.

### Acceptance Direction
- Baseline first, then set explicit p95 latency target from measured baseline.
- Memory/allocation: no strict reduction target; enforce non-regression.
- Backpressure strategy: bounded queue + drop oldest.
- Required evidence: benchmarks + flamegraphs + tracing metrics.

### Visual Assets
- None provided during shaping.

## Intended Outcomes

- Lower UI/input latency during search.
- Reduced allocation and cloning overhead on hot paths.
- Better resilience under file event bursts.
- Measurable, instrumented performance improvements with clear before/after baselines.
