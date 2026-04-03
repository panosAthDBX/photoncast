# Dependency and Error-Handling Audit

Date: 2026-02-09

## Dependency Audit

- Added optional feature in `crates/photoncast-core/Cargo.toml`:
  - `perf = ["dep:tracing-subscriber", "dep:tracing-flame"]`
- Added optional dependencies:
  - `tracing-subscriber` (workspace dependency, optional)
  - `tracing-flame` (optional)
- No new mandatory runtime dependency introduced.

## Error-Handling Audit (Touched Paths)

Reviewed modified runtime paths for panic-prone flow additions:

- `crates/photoncast-core/src/search/file_search_backend.rs`
  - Uses explicit match handling for timeout/errors.
  - No new `unwrap()`/`expect()` in production code paths.
- `crates/photoncast-core/src/search/providers/files.rs`
  - Backend integration and cache operations use non-panicking control flow.
- `crates/photoncast/src/launcher/search.rs`
  - Debounce/cancellation instrumentation uses guarded checks.
- `crates/photoncast-core/src/search/engine.rs`
  - Async provider orchestration with guarded cancellation and task error handling.
- `crates/photoncast-core/src/indexer/watcher.rs`
  - Backpressure and watcher instrumentation paths use non-panicking flow.

Test-only code still contains `expect()` in unit tests (acceptable for tests).

## Conclusion

- Crate-first guidance is respected for this implementation.
- No new panic-driven control flow introduced in touched runtime/library paths.
