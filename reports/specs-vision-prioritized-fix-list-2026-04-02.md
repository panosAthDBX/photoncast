# PhotonCast Prioritized Fix List — 2026-04-02

Derived from: `reports/specs-vision-match-audit-2026-04-02.md`

## Completed on 2026-04-02

### ✅ 1. Correct README hotkey implementation detail
- README now reflects the shipped Carbon-based hotkey implementation.

### ✅ 2. Separate Raycast ecosystem claims from shipped reality
- Mission positioning now distinguishes shipped native extensions from planned Raycast/store work.

### ✅ 3. Implement real window overlay feedback / remove stale claim
- README no longer overclaims, and `crates/photoncast-window/src/overlay.rs` now implements real overlay feedback.

### ✅ 4. Align calendar Join Meeting behavior with the Phase 2 spec
- The Phase 2 spec now matches the shipped Join Meeting behavior.

### ✅ 7. Reconcile shipped first-party extension docs/specs
- The Phase 2 bundled extension list now matches the shipped first-party extension set.

### 🟡 8. Bring mission/README/roadmap/spec language into one consistent “shipped vs planned” format
- Improved substantially in mission/README/specs, but still worth treating as an ongoing hygiene task across future docs.

### ✅ 6. Audit startup wiring for update checking and document it clearly
- Confirmed current state: `photoncast-core` contains the update subsystem, but startup auto-check and the menu-bar manual action are not yet wired end-to-end in the app shell.
- Documented this in `docs/SPARKLE_INTEGRATION.md` and refreshed the audit report accordingly.

## Remaining prioritized work

## P2 — Strengthen proof for important mission claims

### 5. Add explicit verification for activation/hotkey/120 FPS claims
- **Status:** partially improved on 2026-04-02
- **What changed:**
  - added `test_search_with_200_apps_under_30ms_smoke` in `crates/photoncast-core/src/app/integration.rs`
  - added `test_app_initialization_performance_snapshot` and an ignored strict baseline test for `<100ms` init
  - added `reports/performance-evidence-2026-04-02.md`
- **What remains:**
  - hotkey end-to-end `<50ms` still lacks CI-safe automated proof, though a manual proof path now exists via `scripts/run-hotkey-proof.sh`
  - launcher appear-time now has a manual app-shell proof path via `scripts/run-launcher-appear-proof.sh`; the latest same-machine median improved from **~1706 ms** baseline to **~1090 ms**, and then further to **~878 ms** after the latest window-presentation pass, but still does not support the `<50ms` claim
  - 120 FPS proof is still manual/env-gated via `tests/integration/gpui_test.rs`
- **Risk:** medium


### 8. Continue normalizing shipped vs planned language across remaining docs
- **Problem:** cross-doc drift is reduced but not permanently solved.
- **Why it matters:** improves maintainability and future audits.
- **Suggested fix:** keep using explicit labels such as `shipped`, `planned`, and `future/draft` in roadmap/spec refreshes.
- **Risk:** low

## Recommended execution order
1. Strengthen proof and observability: **5**
2. Finish consistency hygiene: **8**

## Suggested next batch
If you want the highest leverage next batch:
- **#5 performance-proof improvements**

That would close the main remaining evidence gap after the doc/spec/runtime alignment work.
