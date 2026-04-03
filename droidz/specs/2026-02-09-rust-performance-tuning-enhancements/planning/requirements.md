# Requirements

## Feature
Rust performance tuning enhancements for PhotonCast.

## Scope
Implement all six recommendations:
1. Prefetch throttling timestamp fix.
2. Move normal-mode search off the UI path (async/debounced execution path).
3. Reduce clone-heavy provider flows (quicklinks/custom commands/files).
4. Add watcher backpressure via bounded queues with drop-oldest policy.
5. Consolidate file-search backend behavior and fallback policy.
6. Add observability metrics and traces for performance validation.

## Rollout
Single implementation batch.

## Acceptance
- p95 search latency target is baseline-driven: capture baseline first, then define concrete target from measured baseline.
- Memory/allocation target: no strict reduction target; require non-regression versus baseline.
- Backpressure policy: bounded queue + drop oldest.
- Observability artifacts required: benchmarks + flamegraphs + tracing metrics.

## Constraints and Standards Alignment
- Align with Rust + Tokio + GPUI architecture and existing provider-based search design.
- Follow crate-first principle before adding any new dependency.
- Prefer async I/O and avoid blocking UI/main-path code.
- Preserve error-handling conventions (`thiserror`/`anyhow`, contextual errors, no panic-driven flow).
- Ensure test coverage includes regression and performance-critical paths.

## Visual Assets
No visual assets were provided during shaping.
