# PRD — Visible Window Presentation

## Objective
Improve **externally visible launcher window presentation** in PhotonCast while preserving current startup semantics.

## Source of Truth / Constraints
- Primary requirements source: `.omx/specs/deep-interview-visible-window-presentation.md`
- Preserve startup semantics; do not broaden lazy/deferred work.
- Focus only on actual window-presentation mechanics.
- Success is a better result in `scripts/run-launcher-appear-proof.sh`.

## In Scope
- Runtime changes to launcher window presentation mechanics
- Manual app-shell harness measurement and internal marker comparison
- Narrow codepaths:
  - `crates/photoncast/src/main.rs`
  - `crates/photoncast/src/launcher/mod.rs`
  - `crates/photoncast/src/launcher/animation.rs`
  - `crates/photoncast/src/platform.rs` when needed for activation/focus behavior
  - supporting harness/report artifacts under `scripts/` and `reports/`

## Out of Scope
- Further startup laziness/deferral changes
- Broad startup optimization outside presentation mechanics
- Unrelated user-visible behavior changes
- Doc-only substitutions for runtime improvement

## Current Evidence Snapshot
- External visible-window result remains far slower than target.
- Internal markers suggest the dominant remaining gap is between window creation/app initialization and externally visible on-screen presentation.

## Acceptance Criteria
1. Only presentation-mechanics codepaths change.
2. Startup semantics remain intact.
3. The latest same-machine baseline is refreshed immediately before the pass.
4. Comparison uses the **median of 3 runs** before/after by default whenever results vary materially.
5. The harness result is measurably better than baseline, or the pass is reported as unsuccessful.
6. Internal markers are compared with external visible time after each pass.
7. Evidence/report artifacts are updated with the measured delta and interpretation.

## Implementation Plan
1. Refresh the latest same-machine launcher-appear baseline.
2. Isolate the presentation-critical path around:
   - ordering / activation behavior
   - window kind / panel semantics
   - initial appear-animation behavior
3. Make one narrow, reversible presentation-only change.
4. Re-run the harness and compare:
   - median visible-window time
   - internal markers
   - whether the dominant gap remains post-`open_launcher_window`
5. Accept only real presentation improvements; reject wins caused by semantic startup drift.
6. Refresh `reports/performance-evidence-2026-04-02.md` and related artifacts.

## Risks / Mitigations
- **Semantic drift disguised as performance win** → explicitly reject changes that alter startup semantics or interaction model.
- **Noisy manual harness** → same-machine baseline refresh + median-of-3 comparison when results vary.
- **macOS/GPUI environment sensitivity** → treat results as directional evidence, not universal truth.
- **Wrong-layer optimization** → require marker-vs-visibility comparison after each pass.

## Verification
- `cargo build -p photoncast -q`
- `scripts/run-launcher-appear-proof.sh`
- compare median-of-3 (or explicitly justified single-run fallback)
- confirm startup semantics unchanged
- update evidence artifacts

## ADR
- **Decision:** execute a narrow presentation-mechanics optimization pass measured by the manual app-shell harness.
- **Drivers:** preserved startup semantics, large internal-vs-external timing gap, user wants visible improvement.
- **Alternatives considered:** broader startup deferral, measurement-only work, generic startup optimization.
- **Why chosen:** best fits the clarified brief and current evidence.
- **Consequences:** may yield incremental rather than dramatic gains; may reveal GPUI/macOS presentation as the true limiting factor.
- **Follow-ups:** if improvement is marginal, refine the harness or add a GPUI-specific manual perf lane before broader changes.

## Available-Agent-Types Roster
- `explore`
- `planner`
- `architect`
- `critic`
- `executor`
- `verifier`
- `writer`
- `team-executor`

## Follow-up Staffing Guidance
### `$ralph`
- `executor` / high — implement one narrow presentation-mechanics change
- `verifier` / high — refresh baseline and compare harness/marker results
- `writer` / medium — refresh evidence artifacts

### `$team`
- Leader: `team-executor`
- Lane A: executor / high — runtime presentation-mechanics change
- Lane B: verifier / high — harness and marker comparison
- Lane C: writer / medium — evidence/report refresh

## Launch Hints
- `$ralph .omx/plans/prd-visible-window-presentation.md`
- `$team .omx/plans/prd-visible-window-presentation.md`
