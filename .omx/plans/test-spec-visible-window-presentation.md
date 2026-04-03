# Test Spec — Visible Window Presentation

## Purpose
Define how execution proves a real improvement in externally visible launcher window presentation without altering startup semantics.

## Preconditions
1. `.omx/plans/prd-visible-window-presentation.md` exists.
2. Execution is constrained to presentation mechanics only.
3. Startup semantics and user interaction model remain fixed.

## Stage 0 — Baseline Refresh
### Must run
- `scripts/run-launcher-appear-proof.sh`

### Must capture
- latest same-machine visible-window baseline
- internal startup markers if present
- whether the result appears noisy enough to require median-of-3 comparison

### Pass conditions
- Baseline is refreshed immediately before the optimization pass.
- If results vary materially, use the **median of 3 runs** before/after.

## Stage 1 — Scope Guard
### Must inspect
- changed codepaths remain inside:
  - `crates/photoncast/src/main.rs`
  - `crates/photoncast/src/launcher/mod.rs`
  - `crates/photoncast/src/launcher/animation.rs`
  - `crates/photoncast/src/platform.rs` when needed

### Pass conditions
- No broad startup deferral/laziness changes introduced.
- No unrelated user-visible startup behavior changes introduced.

## Stage 2 — Buildability
### Must run
- `cargo build -p photoncast -q`

### Pass conditions
- Build succeeds.

## Stage 3 — Improvement Proof
### Must run
- `scripts/run-launcher-appear-proof.sh` after the change

### Must compare
- pre-change vs post-change visible-window result
- internal markers pre vs post
- whether the dominant gap still appears after `open_launcher_window`

### Pass conditions
- Post-change result is measurably better than refreshed baseline.
- If not better, the pass is unsuccessful unless a clearly documented harness correction explains the delta.

## Stage 4 — Rejection Rule
The pass fails if improvement comes primarily from:
- new deferred startup work
- altered startup semantics
- changed intended interaction model rather than presentation-only mechanics

## Stage 5 — Evidence Refresh
### Must update
- `reports/performance-evidence-2026-04-02.md`
- optionally `reports/specs-vision-prioritized-fix-list-2026-04-02.md`

### Pass conditions
- Evidence reflects the latest measured result and explains whether the remaining gap is in:
  - app initialization,
  - window creation,
  - or external presentation timing.
