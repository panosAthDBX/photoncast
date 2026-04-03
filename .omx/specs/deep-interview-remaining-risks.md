# Deep Interview Spec — Remaining Risks

## Metadata
- Profile: standard
- Rounds: 3
- Final ambiguity: 8%
- Threshold: 20%
- Context type: brownfield
- Context snapshot: `.omx/context/remaining-risks-20260403T124434Z.md`
- Transcript: `.omx/interviews/remaining-risks-20260403T124730Z.md`

## Clarity Breakdown
| Dimension | Score |
|---|---:|
| Intent | 90% |
| Outcome | 90% |
| Scope | 90% |
| Constraints | 82% |
| Success Criteria | 82% |
| Context Clarity | 90% |

## Intent
Produce a durable, product/codebase-level understanding of PhotonCast's remaining risks.

## Desired Outcome
Create a **ranked risk register** for the PhotonCast product/codebase overall.

## In Scope
- Durable risks to PhotonCast as a product/codebase
- Risks grounded in current repo evidence and recent audit artifacts
- Risks such as:
  - performance-evidence gaps
  - app-shell update wiring gaps
  - GPUI/window-presentation uncertainty
  - ongoing doc/spec consistency risks

## Out of Scope / Non-goals
- Temporary local-worktree state
- Temporary install-state issues specific to a single local machine
- A go/no-go release recommendation unless requested later

## Decision Boundaries
- Rank only durable risks to PhotonCast itself.
- Exclude ephemeral local state from the main register.
- OMX may group and prioritize risks by severity/likelihood based on repo evidence.

## Constraints
- Brownfield evidence only
- No direct implementation in deep-interview mode
- Output should be a ranked register, not just narrative discussion

## Testable Acceptance Criteria
1. The output is a ranked risk register.
2. It excludes temporary local-worktree/install-state issues.
3. It focuses on durable product/codebase risks.
4. Each risk is grounded in current repo/report evidence.

## Assumptions Exposed + Resolutions
- Assumption: “remaining risks” might mean only the current local build.  
  Resolution: it means the PhotonCast product/codebase overall.
- Assumption: the user might want a go/no-go recommendation.  
  Resolution: they want a ranked risk register.
- Assumption: temporary local state might belong in the output.  
  Resolution: explicitly exclude it.

## Pressure-Pass Findings
- Revisited the scope boundary and confirmed that only durable risks belong in the output.

## Brownfield Evidence vs Inference Notes
- Evidence: current reports and code show remaining performance-proof gaps, update-wiring gaps, and presentation uncertainty.
- Inference: these are durable enough to rank at the product/codebase level.

## Technical Context Findings
Primary likely touchpoints:
- `reports/specs-vision-match-audit-2026-04-02.md`
- `reports/specs-vision-prioritized-fix-list-2026-04-02.md`
- `reports/performance-evidence-2026-04-02.md`
- `docs/SPARKLE_INTEGRATION.md`
