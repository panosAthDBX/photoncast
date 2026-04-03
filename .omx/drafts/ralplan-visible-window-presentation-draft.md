# Consensus Plan Draft — Visible Window Presentation

## Requirements Summary
- Improve **externally visible launcher window presentation** in PhotonCast.
- Preserve **current startup semantics**; do not widen lazy/deferred work beyond what is already changed.
- Focus on **window-presentation mechanics only**.
- Success is a **measurably better result** in `scripts/run-launcher-appear-proof.sh`, not necessarily hitting the final target in one pass.

### Grounding evidence
- Deep-interview source of truth: `.omx/specs/deep-interview-visible-window-presentation.md`
- Current manual harness: `scripts/run-launcher-appear-proof.sh`
- Latest evidence: `reports/performance-evidence-2026-04-02.md`
- Current startup path: `crates/photoncast/src/main.rs:240-360`
- Launcher creation path: `crates/photoncast/src/main.rs:883-913`
- Launcher initialization path: `crates/photoncast/src/launcher/mod.rs:579-711`
- Launcher appear animation path: `crates/photoncast/src/launcher/animation.rs:1-61`

### Current evidence snapshot
- Manual launcher visible time: refresh to the latest same-machine baseline immediately before execution; most recently observed baseline was ~998ms in the current evidence trail.
- Internal markers from the harness show:
  - main start → before `open_launcher_window`: ~155ms
  - `open_launcher_window`: ~64ms
  - after `open_launcher_window` → app initialized: ~13ms
- Inference: the dominant remaining gap is likely between app/window creation and **observable on-screen presentation**, not broad Rust startup work.

## Acceptance Criteria
1. The implementation changes only **window-presentation mechanics** and preserves current startup semantics.
2. The implementation touches only the relevant launcher/window-presentation path unless a dependent helper is strictly required.
3. `scripts/run-launcher-appear-proof.sh` shows a **measurable improvement** against a concrete baseline: refresh the latest same-machine baseline immediately before execution and compare the **median of 3 runs** before vs after whenever the result is noisy; if only one rerun is practical, report that limitation explicitly.
4. Any change to activation behavior or window kind/panel semantics is acceptable only if it does **not** alter the intended startup semantics or user interaction model.
5. The final report explains whether the improvement came from:
   - faster GPUI window presentation,
   - different focus/activation/window-kind behavior,
   - or measurement refinement.
6. If the change does not improve the harness, it is not accepted as complete.
7. Verification includes buildability plus rerunning the manual app-shell harness and comparing internal markers with external visible time after each pass.

## Implementation Steps
1. **Isolate the presentation-critical path**
   - Inspect:
     - `crates/photoncast/src/main.rs` (`open_launcher_window`, initial window options)
     - `crates/photoncast/src/launcher/mod.rs` (`LauncherWindow::new`, `show`)
     - `crates/photoncast/src/launcher/animation.rs` (appear animation state transitions)
     - `crates/photoncast/src/platform.rs` only if activation/focus behavior is involved
   - Determine whether the primary cost is:
     - delayed key/main ordering,
     - nonactivating popup behavior,
     - initial appear animation/state,
     - or visibility measurement mismatch.

2. **Choose the narrowest presentation-only option**
   Candidate implementation options to evaluate:
   - **Option A:** adjust initial launcher window presentation parameters in `open_launcher_window` (focus/show/kind/background/ordering related) without changing semantic startup work.
   - **Option B:** tighten the initial `LauncherWindow::new` / appear-animation path so the first visible frame is presented earlier, while keeping the same later behavior.
   - **Option C:** refine the manual harness and internal markers further if evidence still suggests a measurement artifact dominates.
   - Avoid any change whose primary benefit comes from deferring more work.

3. **Implement one presentation-mechanics pass only**
   - Make the smallest reversible code change in the selected path.
   - Keep write scope narrow and document why the change is presentation-only.
   - Do not alter feature readiness semantics, extension behavior, or background subsystem sequencing beyond existing state.

4. **Re-measure with the manual harness**
   - Run `scripts/run-launcher-appear-proof.sh` after the change.
   - Use the latest same-machine baseline as authoritative, and use the **median of 3 runs** before/after by default whenever results vary materially.
   - Capture:
     - visible window time
     - internal markers if present
     - comparison against the pre-change baseline
     - whether the gap still appears primarily after `open_launcher_window`
   - If the result is not better, iterate once on the same narrow path before considering the pass unsuccessful.

5. **Refresh evidence artifacts**
   - Update `reports/performance-evidence-2026-04-02.md` with the new measurement and interpretation.
   - If appropriate, update `reports/specs-vision-prioritized-fix-list-2026-04-02.md` to reflect the new state of item #5.

## Risks and Mitigations
- **Risk: accidental startup-semantics drift.**  
  **Mitigation:** reject options that primarily win by deferring more work or changing readiness semantics.
- **Risk: measurement artifact mistaken for runtime improvement.**  
  **Mitigation:** keep both external visible-time measurement and internal markers; report both.
- **Risk: GPUI/macOS presentation behavior is inherently environment-sensitive.**  
  **Mitigation:** treat harness improvement as directional proof, not universal truth.
- **Risk: popup/nonactivating window behavior may have product side effects.**  
  **Mitigation:** constrain changes to the launcher’s initial presentation path and verify behavior manually; reject any change that alters the intended user interaction model.

## Verification Steps
1. Build the affected target(s): at minimum `cargo build -p photoncast -q`.
2. Re-run `scripts/run-launcher-appear-proof.sh`.
3. Confirm the measured visible-window time is lower than the current baseline.
4. Confirm startup semantics were not intentionally broadened (no new deferred startup work introduced).
5. Summarize the delta in `reports/performance-evidence-2026-04-02.md`.

## RALPLAN-DR Summary
### Principles
1. **Presentation-only optimization** — do not solve this by broadening startup laziness.
2. **Measure externally visible behavior** — optimize the user-observable outcome, not just internal markers.
3. **Prefer the narrowest reversible change** that improves visible presentation.
4. **Evidence before interpretation** — keep harness measurements and internal markers together, and revalidate the gap location after each pass.
5. **Preserve startup semantics** as a hard boundary.

### Decision Drivers
1. The user explicitly wants to preserve startup semantics.
2. The current evidence shows a large gap between internal timing and external visibility.
3. Success is defined as a better harness result, not an arbitrary architectural rewrite.

### Viable Options
#### Option A — Adjust initial window presentation parameters in a staged order **(recommended)**
- **Approach:** tune `open_launcher_window` / initial GPUI window options in this order:
  1. ordering / activation behavior
  2. window kind / panel semantics
  3. initial appear-animation behavior
- **Pros:** most directly targets the observed gap; preserves startup work semantics; easy to stop after first measurable win.
- **Cons:** macOS/GPUI behavior may be subtle and environment-sensitive, and activation/window-kind changes must not alter the intended user interaction model.

#### Option B — Tighten the initial appear-animation path
- **Approach:** change first-frame/animation behavior so the launcher becomes visibly present sooner.
- **Pros:** directly affects visible presentation; small write scope.
- **Cons:** may have less impact if the bottleneck is before first composited frame.

#### Option C — Measurement refinement only
- **Approach:** improve harnesses/markers without changing runtime behavior.
- **Pros:** lowers uncertainty.
- **Cons:** does not satisfy the user’s desire to “work on visible window presentation” unless runtime improvement is impossible.

### Recommendation
Start with **Option A**, because the current evidence points to a gap after `open_launcher_window` but before externally visible presentation, making initial window presentation mechanics the highest-leverage path.

## ADR
- **Decision:** Plan one narrow pass focused on launcher window presentation mechanics, measured by the manual app-shell harness.
- **Drivers:** preserved startup semantics; large discrepancy between internal and external timing; user wants visible improvement.
- **Alternatives considered:** broader startup deferral (rejected by user), measurement-only pass, generic startup optimization.
- **Why chosen:** best matches the clarified brief and current evidence.
- **Consequences:** may produce only incremental gains; further passes may still be needed if GPUI/macOS presentation is the limiting factor.
- **Follow-ups:** if no meaningful gain appears, reassess whether a GPUI-specific manual harness is needed before more runtime changes.

## Available-Agent-Types Roster
- `explore` — codepath mapping and hypothesis gathering
- `planner` — plan refinement / sequencing
- `architect` — review boundaries and presentation-only correctness
- `critic` — validate testability and evidence quality
- `executor` — implement the narrow presentation mechanics change
- `verifier` — rerun harnesses and compare measurements
- `writer` — refresh evidence/report artifacts

## Follow-up Staffing Guidance
### If using `$ralph`
- `executor` / high — implement the presentation-only change
- `verifier` / high — rerun harness and compare measurements
- `writer` / medium — refresh evidence artifacts if the metric changes

### If using `$team`
- Leader: `team-executor`
- Lane A: `executor` / high — runtime presentation-mechanics change
- Lane B: `verifier` / high — harness reruns and measurement comparison
- Lane C: `writer` / medium — evidence/report refresh

## Launch Hints
- `$ralph .omx/specs/deep-interview-visible-window-presentation.md`
- `$team .omx/specs/deep-interview-visible-window-presentation.md`

## Team Verification Path
1. Confirm only presentation-mechanics codepaths changed.
2. Confirm no new startup laziness/semantic drift was introduced.
3. Re-run the manual app-shell harness and compare against baseline.
4. Update evidence/reporting with the measured delta.
5. Reject any change that improves the harness only by effectively altering startup semantics or unrelated user-visible startup behavior.


## Applied Improvements
- Added an explicit verification step to confirm the dominant gap still appears after `open_launcher_window` after each pass.
- Narrowed the recommended implementation option into a staged sequence: ordering/activation, then window kind/panel semantics, then initial appear-animation behavior.
- Added an explicit rejection rule for any apparent win that comes from semantic startup drift rather than presentation mechanics.


## Applied Improvements (Iteration 2)
- Defined a concrete comparison rule for the manual harness: same-machine baseline, median-of-3 preferred when feasible.
- Made the activation/window-kind guardrail explicit: acceptable only if startup semantics and user interaction model stay intact.
- Strengthened verification to require marker-vs-visibility comparison after each pass, not only final visible time.


## Applied Improvements (Iteration 3)
- Made refreshing the latest same-machine baseline an explicit pre-execution requirement.
- Tightened the median-of-3 rule from optional preference to the default comparison method whenever results are noisy.
