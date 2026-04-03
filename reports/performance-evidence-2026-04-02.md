# PhotonCast Performance Evidence — 2026-04-02

## Goal
Clarify which headline performance claims are directly evidenced in code/tests today, which have partial proof, and which still require manual or stronger automated verification.

## Directly evidenced

### Search latency < 30ms
- Existing direct assertion:
  - `tests/integration/search_test.rs` → `test_search_performance_target`
- Added additional core smoke proof:
  - `crates/photoncast-core/src/app/integration.rs` → `test_search_with_200_apps_under_30ms_smoke`
- Verification run on 2026-04-02:
  - `cargo test -p photoncast-core test_search_with_200_apps_under_30ms_smoke -- --nocapture` ✅

## Partially evidenced

### App initialization / cold-start target < 100ms
- Existing snapshot-style evidence:
  - `tests/integration/e2e_test.rs` → `test_app_initialization_performance`
- Added core evidence hooks:
  - `crates/photoncast-core/src/app/integration.rs` → `test_app_initialization_performance_snapshot`
  - `crates/photoncast-core/src/app/integration.rs` → `test_app_initialization_under_100ms_strict` (`#[ignore]`, manual baseline)
- Verification run on 2026-04-02:
  - `cargo test -p photoncast-core test_app_initialization_performance_snapshot -- --nocapture` ✅
  - observed snapshots on this machine/session ranged from **~70ms to ~101ms** across repeated runs
- Interpretation:
  - the target can be met on this machine, but evidence is still too variable and environment-sensitive to treat `<100ms` as universally proven across representative environments.

## Still not directly automated end-to-end

### Hotkey response < 50ms
- There is functional coverage for hotkey behavior and conflict detection:
  - `tests/integration/hotkey_test.rs`
- There is now also a **manual proof path** in the app-shell target:
  - `crates/photoncast/src/platform.rs` contains ignored hotkey callback latency tests
  - `scripts/run-hotkey-proof.sh` runs that manual proof path
- But there is still **no CI-safe direct end-to-end latency test** proving keypress-to-visible-response under 50ms in the shipped app shell.
- Current status: **manual proof path exists, but not yet fully automated/proven in a generic unit-test environment**.

### 120 FPS rendering / <50ms launcher appear
- `tests/integration/gpui_test.rs` contains the intended manual/ignored GPUI performance checks:
  - `test_120fps_baseline`
  - `test_window_appear_time`
- Added reusable manual launcher-appear harness:
  - `scripts/run-launcher-appear-proof.sh`
- Verification run on **2026-04-03**:
  - refreshed same-machine baseline (median of 3 runs before this pass): **~1706 ms**
  - previous median before the current pass: **~1090 ms**
  - latest median of 3 runs after switching the launcher to a normal window for the initial presentation path: **~878 ms**
  - post-change internal markers remained roughly similar:
    - `main_start` → `before_open_launcher_window`: ~164-175 ms
    - `before_open_launcher_window` → `after_open_launcher_window`: ~59-69 ms
    - `after_open_launcher_window` → `app_initialized`: ~12-21 ms
- Interpretation:
  - this is a real manual app-shell proof path, and it still does **not** support the `<50ms` launcher-appear claim in its current form.
  - the internal markers suggest PhotonCast reaches `open_window` and app initialization much faster than the external visibility measurement, which points to a remaining gap between app initialization and observable on-screen window visibility.
  - that makes the next optimization/measurement problem more specifically about **window visibility timing / app activation / GPUI window presentation**, not just generic Rust startup work.

## What improved in this batch
- Added a new passing core smoke test for `<30ms` search on a realistic 200-app fixture.
- Added a startup performance snapshot test plus a strict ignored baseline test for `<100ms` app initialization.
- Added a manual hotkey proof path via `scripts/run-hotkey-proof.sh`.
- Added a manual launcher-appear proof path via `scripts/run-launcher-appear-proof.sh`.
- Improved the launcher appear median from ~1706 ms baseline into the ~900-1100 ms range by making the launcher immediately visible while scale animation continues.
- Improved it further from ~1090 ms to ~878 ms by testing normal-window presentation instead of popup-panel semantics on the initial launcher path.
- Converted the remaining performance gap from “unclear” to “explicitly categorized”: proven, partial, manual-only, or currently failing against target.

## Recommended next steps
1. Add a representative hotkey end-to-end latency measurement path.
2. Replace the ignored GPUI performance stubs with runnable manual harnesses or a dedicated perf lane.
3. Decide whether mission/roadmap language should say **target** vs **achieved** for cold-start/120 FPS until stronger proof exists.

## Reusable proof path
- `scripts/run-performance-evidence.sh` runs the currently available automated checks and prints the remaining manual/environment-gated steps.
- `scripts/run-hotkey-proof.sh` runs the current manual hotkey proof path in the photoncast app-shell target.
- `scripts/run-launcher-appear-proof.sh` runs the current manual launcher-appear app-shell proof path and prints internal startup markers when available.
