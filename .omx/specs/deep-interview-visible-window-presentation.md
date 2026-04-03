# Deep Interview Spec — Visible Window Presentation

## Metadata
- Profile: standard
- Rounds: 3
- Final ambiguity: 8%
- Threshold: 20%
- Context type: brownfield
- Context snapshot: `.omx/context/visible-window-presentation-20260403T111549Z.md`
- Transcript: `.omx/interviews/visible-window-presentation-20260403T111730Z.md`

## Clarity Breakdown
| Dimension | Score |
|---|---:|
| Intent | 92% |
| Outcome | 90% |
| Scope | 90% |
| Constraints | 90% |
| Success Criteria | 82% |
| Context Clarity | 90% |

## Intent
Improve the time to **externally visible launcher window presentation** in PhotonCast.

## Desired Outcome
Achieve a **visibly better result in the manual app-shell harness** for launcher visibility timing.

## In Scope
- Runtime changes specifically targeting window-presentation mechanics
- Measurement against the existing manual app-shell launcher-appear harness
- Narrow brownfield changes to the relevant launcher/window-presentation path

## Out of Scope / Non-goals
- Changing startup semantics by deferring more work
- Broad startup optimization outside visible window presentation mechanics
- User-visible product-behavior changes unrelated to presentation timing
- Doc-only fixes as a substitute for runtime improvement

## Decision Boundaries
- Preserve current startup semantics.
- Optimize only actual window-presentation mechanics.
- OMX may choose among implementation approaches inside that boundary if they do not broaden scope or alter product semantics.

## Constraints
- Brownfield repo optimization only
- Must preserve behavior semantics
- Must use the manual app-shell harness as the success measure for this pass
- No broad deferral/laziness changes

## Testable Acceptance Criteria
1. The implementation targets window-presentation mechanics specifically.
2. Startup semantics are preserved.
3. The manual app-shell harness result is measurably better than the prior baseline.
4. Verification evidence includes the harness output and a concise explanation of what changed.

## Assumptions Exposed + Resolutions
- Assumption: earlier visible presentation could be achieved by deferring more startup work.  
  Resolution: explicitly rejected for this pass.
- Assumption: success requires hitting a hard latency target immediately.  
  Resolution: success is improvement in the manual app-shell harness, not necessarily hitting the final target in one pass.

## Pressure-Pass Findings
- Revisited the optimization strategy and confirmed that semantic changes / extra laziness are out of bounds even if they might improve the metric.

## Brownfield Evidence vs Inference Notes
- Evidence: current manual harness and internal markers suggest the remaining gap is specifically around external window visibility timing.
- Inference: the next execution pass should focus on presentation mechanics rather than more startup reordering.

## Technical Context Findings
Primary likely touchpoints:
- `crates/photoncast/src/main.rs`
- `crates/photoncast/src/launcher/mod.rs`
- `crates/photoncast/src/platform.rs`
- manual proof harness scripts under `scripts/`
- evidence artifacts under `reports/`
